use crate::error::DataAccountError;
use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    keccak, 
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    secp256k1_recover::secp256k1_recover,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

pub struct TypeUtils;
pub struct DataAccountUtils;

impl TypeUtils {
    pub fn eth_address_from_pubkey(pk: [u8; 64]) -> [u8; 20] {
        let hash = keccak::hash(&pk).to_bytes();
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..32]);
        address
    }

    pub fn recover_eth_address(message: &[u8], signature: [u8; 64]) -> [u8; 20] {
        let digest = keccak::hash(&message).to_bytes();

        let mut signature_split = [0 as u8; 64];
        signature_split.copy_from_slice(&signature);
        let first_bit_of_s = signature_split.get_mut(32).unwrap();
        let recovery_id = *first_bit_of_s >> 7;
        *first_bit_of_s = *first_bit_of_s & 0x7f;

        let pubkey = secp256k1_recover(&digest, recovery_id, &signature_split);
        match pubkey {
            Ok(eth_pubkey) => Self::eth_address_from_pubkey(eth_pubkey.to_bytes()),
            Err(_error) => [0; 20],
        }
    }
}

impl DataAccountUtils {
    /// Creates a Program Derived Address (PDA) account with specified parameters
    /// 
    /// # Arguments
    /// * `program_id` - The program that will own the account
    /// * `payer_account` - Account that will pay for the new account creation
    /// * `target_account` - Account to be created as a PDA
    /// * `prefix` - Seed prefix for PDA derivation
    /// * `phrase` - Additional seed for PDA derivation
    /// * `data_length` - Size of the account data in bytes
    pub fn create_related_account<'a>(
        program_id: &Pubkey,
        payer_account: &AccountInfo<'a>,
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
                &system_instruction::create_account(
                    payer_account.key,
                    target_account.key,
                    required_lamports,
                    data_length as u64,
                    program_id,
                ),
                &[payer_account.clone(), target_account.clone()],
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
}