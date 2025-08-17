use borsh::{BorshDeserialize, BorshSerialize};
use std::{cmp::Ordering, collections::HashSet};

use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    keccak,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    secp256k1_recover::secp256k1_recover,
    sysvar::{rent::Rent, Sysvar},
};
use solana_system_interface::instruction::create_account;

use crate::{
    constants::{Constants, EthAddress},
    error::{DataAccountError, FreeTunnelError},
    state::{BasicStorage, ExecutorsInfo},
};

pub struct SignatureUtils;
pub struct DataAccountUtils;

impl SignatureUtils {
    pub(crate) fn log10(n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            (n as f64).log10().floor() as u64
        }
    }

    pub(crate) fn join_address_list(eth_addrs: &Vec<EthAddress>) -> Vec<u8> {
        let mut result = Vec::new();
        for addr in eth_addrs {
            result.extend_from_slice(b"0x");
            result.extend_from_slice(hex::encode(addr).as_bytes());
            result.extend_from_slice(b"\n");
        }
        result
    }

    pub(crate) fn cmp_addr_list(list1: &Vec<EthAddress>, list2: &Vec<EthAddress>) -> bool {
        match list1.len().cmp(&list2.len()) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => list1
                .iter()
                .zip(list2.iter())
                .find_map(|(a, b)| match a.cmp(b) {
                    Ordering::Greater => Some(true),
                    Ordering::Less => Some(false),
                    Ordering::Equal => None,
                })
                .unwrap_or(false),
        }
    }

    pub(crate) fn check_executors_not_duplicated(executors: &[EthAddress]) -> ProgramResult {
        let mut seen = HashSet::new();
        match executors.iter().all(|addr| seen.insert(addr)) {
            true => Ok(()),
            false => Err(FreeTunnelError::DuplicatedExecutors.into()),
        }
    }

    pub(crate) fn eth_address_from_pubkey(pk: [u8; 64]) -> EthAddress {
        let hash = keccak::hash(&pk).to_bytes();
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..32]);
        address
    }

    pub(crate) fn recover_eth_address(message: &[u8], mut signature: [u8; 64]) -> EthAddress {
        let digest = keccak::hash(&message).to_bytes();

        let first_bit_of_s = signature.get_mut(32).unwrap();
        let recovery_id = *first_bit_of_s >> 7;
        *first_bit_of_s = *first_bit_of_s & 0x7f;

        let pubkey = secp256k1_recover(&digest, recovery_id, &signature);
        match pubkey {
            Ok(eth_pubkey) => Self::eth_address_from_pubkey(eth_pubkey.to_bytes()),
            Err(_error) => [0; 20],
        }
    }

    fn check_signature(
        message: &[u8],
        signature: [u8; 64],
        eth_signer: EthAddress,
    ) -> ProgramResult {
        match eth_signer == Constants::ETH_ZERO_ADDRESS {
            true => Err(FreeTunnelError::SignerCannotBeZeroAddress.into()),
            false => {
                let recovered_eth_addr = Self::recover_eth_address(message, signature);
                match recovered_eth_addr == eth_signer {
                    true => Ok(()),
                    false => Err(FreeTunnelError::InvalidSignature.into()),
                }
            }
        }
    }

    fn check_executors_for_index(
        data_account_basic_storage: &AccountInfo,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check executors threshold
        let ExecutorsInfo {
            index: _,
            threshold,
            active_since,
            executors: current_executors,
        } = DataAccountUtils::read_account_data(data_account_current_executors)?;
        if executors.len() < threshold as usize {
            return Err(FreeTunnelError::NotMeetThreshold.into());
        }

        // Check timestamp for current index
        let now = Clock::get()?.unix_timestamp;
        if now < (active_since as i64) {
            return Err(FreeTunnelError::ExecutorsNotYetActive.into());
        }

        // Check timestamp for next index
        let BasicStorage {
            executors_group_length,
            ..
        } = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if executors_group_length > exe_index + 1 {
            let ExecutorsInfo {
                active_since: next_active_since,
                ..
            } = DataAccountUtils::read_account_data(data_account_next_executors)?;
            if now > (next_active_since as i64) {
                return Err(FreeTunnelError::ExecutorsOfNextIndexIsActive.into());
            }
        }

        // Check executors index
        for (i, executor) in executors.iter().enumerate() {
            if executors[0..i].iter().any(|e| e == executor) {
                return Err(FreeTunnelError::DuplicatedExecutors.into());
            }
            if !current_executors.iter().any(|e| e == executor) {
                return Err(FreeTunnelError::NonExecutors.into());
            }
        }

        Ok(())
    }

    pub(crate) fn check_multi_signatures(
        data_account_basic_storage: &AccountInfo,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        message: &[u8],
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        if signatures.len() != executors.len() {
            return Err(FreeTunnelError::ArrayLengthNotEqual.into());
        }
        Self::check_executors_for_index(
            data_account_basic_storage,
            data_account_current_executors,
            data_account_next_executors,
            executors,
            exe_index,
        )?;

        for (i, executor) in executors.iter().enumerate() {
            Self::check_signature(message, signatures[i], *executor)?;
        }
        Ok(())
    }
}

impl DataAccountUtils {
    /// Creates a Program Derived Address (PDA) account with specified parameters
    ///
    /// # Arguments
    /// * `program_id` - The program that will own the account
    /// * `account_payer` - Account that will pay for the new account creation
    /// * `target_account` - Account to be created as a PDA
    /// * `prefix` - Seed prefix for PDA derivation
    /// * `phrase` - Additional seed for PDA derivation
    /// * `data_length` - Size of the account data in bytes
    pub fn create_related_account<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        target_account: &AccountInfo<'a>,
        prefix: &[u8],
        phrase: &[u8],
        data_length: usize,
    ) -> ProgramResult {
        let (pda_pubkey, bump_seed) = Pubkey::find_program_address(&[prefix, phrase], program_id);
        if pda_pubkey != *target_account.key {
            Err(DataAccountError::PdaAccountMismatch.into())
        } else if !target_account.is_writable {
            Err(DataAccountError::PdaAccountNotWritable.into())
        } else if !target_account.data_is_empty() {
            Err(DataAccountError::PdaAccountAlreadyCreated.into())
        } else {
            let rent = Rent::get()?;
            let required_lamports = rent.minimum_balance(data_length);
            invoke_signed(
                &create_account(
                    account_payer.key,
                    target_account.key,
                    required_lamports,
                    data_length as u64,
                    program_id,
                ),
                &[account_payer.clone(), target_account.clone()],
                &[&[prefix.as_ref(), phrase.as_ref(), &[bump_seed]]],
            )
        }
    }

    pub fn check_account_ownership(program_id: &Pubkey, account: &AccountInfo) -> ProgramResult {
        match account.owner == program_id {
            true => Ok(()),
            false => Err(DataAccountError::PdaAccountNotOwned.into()),
        }
    }

    pub fn check_account_match(
        program_id: &Pubkey,
        account: &AccountInfo,
        prefix: &[u8],
        phrase: &[u8],
    ) -> ProgramResult {
        let (pda_pubkey, _) = Pubkey::find_program_address(&[prefix, phrase], program_id);
        match account.key == &pda_pubkey {
            true => Ok(()),
            false => Err(DataAccountError::PdaAccountMismatch.into()),
        }
    }

    pub fn write_account_data<Data: BorshSerialize>(
        data_account: &AccountInfo,
        content: Data,
    ) -> ProgramResult {
        let account_data = &mut data_account.data.borrow_mut()[..];
        let mut buffer = Vec::new();
        content
            .serialize(&mut buffer)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        account_data[..4].copy_from_slice(&(buffer.len() as u32).to_le_bytes());
        account_data[4..4 + buffer.len()].copy_from_slice(&buffer);
        Ok(())
    }

    pub fn read_account_data<Data: BorshDeserialize>(
        data_account: &AccountInfo,
    ) -> Result<Data, ProgramError> {
        let account_data = &data_account.data.borrow()[..];
        let data_len = u32::from_le_bytes(account_data[..4].try_into().unwrap()) as usize;
        Data::try_from_slice(&account_data[4..4 + data_len])
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn is_empty_account(data_account: &AccountInfo) -> bool {
        data_account.data_is_empty()
    }
}
