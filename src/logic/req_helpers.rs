use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};
use spl_token::state::{Account as TokenAccount, GenericTokenAccount};
use spl_token_2022::{
    state::Account as Token2022Account,
    generic_token_account::GenericTokenAccount as GenericToken2022Account,
};

use crate::error::FreeTunnelError;
use crate::state::BasicStorage;
use crate::utils::DataAccountUtils;
use crate::constants::Constants;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ReqId {
    /// In format of: `version:uint8|createdTime:uint40|action:uint8`
    ///     + `tokenIndex:uint8|amount:uint64|from:uint8|to:uint8|(TBD):uint112`
    pub data: [u8; 32],
}

impl ReqId {
    pub fn new(data: [u8; 32]) -> Self {
        Self { data }
    }

    pub fn version(&self) -> u8 {
        self.data[0]
    }

    pub fn created_time(&self) -> u64 {
        let mut time = 0;
        for i in 1..6 {
            time = (time << 8) + self.data[i] as u64;
        }
        time
    }

    pub fn checked_created_time(&self) -> Result<u64, ProgramError> {
        let time = self.created_time();
        let now = Clock::get()?.unix_timestamp;
        if ((time + Constants::PROPOSE_PERIOD) as i64) <= now {
            Err(FreeTunnelError::CreatedTimeTooEarly.into())
        } else if (time as i64) >= now + 60 {
            Err(FreeTunnelError::CreatedTimeTooLate.into())
        } else { Ok(time) }
    }

    pub fn action(&self) -> u8 {
        self.data[6]
    }

    pub fn token_index(&self) -> u8 {
        self.data[7]
    }

    pub fn get_checked_token<'a>(
        &self,
        data_account_basic_storage: &AccountInfo<'a>,
        token_account: Option<&AccountInfo<'a>>,
    ) -> Result<(u8, u8), ProgramError> {
        let BasicStorage {
            tokens, decimals, ..
        } = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        let token_index = self.token_index();
        let token_pubkey = tokens.get(token_index).ok_or(FreeTunnelError::TokenIndexNonExistent)?;
        let decimal = decimals.get(token_index).ok_or(FreeTunnelError::TokenIndexNonExistent)?;
        if *token_pubkey == Pubkey::default() {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else {
            if let Some(token_account) = token_account {
                let token_account_data = token_account.data.borrow();
                if token_account.owner == &spl_token::id() {
                    match TokenAccount::valid_account_data(&token_account_data) {
                        true => {
                            let token_mint = TokenAccount::unpack_account_mint_unchecked(&token_account_data);
                            if *token_pubkey != *token_mint {
                                return Err(FreeTunnelError::TokenMismatch.into());
                            }
                        }
                        false => return Err(FreeTunnelError::InvalidTokenAccount.into()),
                    }
                } else if token_account.owner == &spl_token_2022::id() {
                    match Token2022Account::valid_account_data(&token_account_data) {
                        true => {
                            let token_mint = Token2022Account::unpack_account_mint_unchecked(&token_account_data);
                            if *token_pubkey != *token_mint {
                                return Err(FreeTunnelError::TokenMismatch.into());
                            }
                        }
                        false => return Err(FreeTunnelError::InvalidTokenAccount.into()),
                    }
                } else {
                    return Err(FreeTunnelError::InvalidTokenAccount.into());
                }
            }
            Ok((token_index, *decimal))
        }
    }

    pub fn raw_amount(&self) -> u64 {
        u64::from_be_bytes(self.data[8..16].try_into().unwrap())
    }

    pub fn get_checked_amount(&self, decimal: u8) -> Result<u64, ProgramError> {
        let mut amount = self.raw_amount();
        if amount == 0 {
            Err(FreeTunnelError::AmountCannotBeZero.into())
        } else if decimal > 6 {
            let factor = Self::checked_pow10((decimal - 6) as u32)?;
            amount = amount.checked_mul(factor).ok_or(FreeTunnelError::ArithmeticOverflow)?;
            Ok(amount)
        } else if decimal < 6 {
            let factor = Self::checked_pow10((6 - decimal) as u32)?;
            amount /= factor;
            if amount == 0 { Err(FreeTunnelError::AmountCannotBeZero.into()) } else { Ok(amount) }
        } else { Ok(amount) }
    }

    fn checked_pow10(exp: u32) -> Result<u64, ProgramError> {
        let mut value = 1u64;
        for _ in 0..exp {
            value = value.checked_mul(10).ok_or(FreeTunnelError::ArithmeticOverflow)?;
        }
        Ok(value)
    }

    pub fn msg_from_req_signing_message(&self) -> Vec<u8> {
        let specific_action = self.action() & 0x0f;
        let mut msg = Constants::ETH_SIGN_HEADER.to_vec();
        match specific_action {
            1 => {
                let length = 3 + Constants::BRIDGE_CHANNEL.len() + 29 + 66;
                msg.extend_from_slice(length.to_string().as_bytes());
                msg.extend_from_slice(b"["); msg.extend_from_slice(Constants::BRIDGE_CHANNEL); msg.extend_from_slice(b"]\n");
                msg.extend_from_slice(b"Sign to execute a lock-mint:\n");
                msg.extend_from_slice(b"0x"); msg.extend_from_slice(hex::encode(&self.data).as_bytes());
                msg
            }
            2 => {
                let length = 3 + Constants::BRIDGE_CHANNEL.len() + 31 + 66;
                msg.extend_from_slice(length.to_string().as_bytes());
                msg.extend_from_slice(b"["); msg.extend_from_slice(Constants::BRIDGE_CHANNEL); msg.extend_from_slice(b"]\n");
                msg.extend_from_slice(b"Sign to execute a burn-unlock:\n");
                msg.extend_from_slice(b"0x"); msg.extend_from_slice(hex::encode(&self.data).as_bytes());
                msg
            }
            3 => {
                let length = 3 + Constants::BRIDGE_CHANNEL.len() + 29 + 66;
                msg.extend_from_slice(length.to_string().as_bytes());
                msg.extend_from_slice(b"["); msg.extend_from_slice(Constants::BRIDGE_CHANNEL); msg.extend_from_slice(b"]\n");
                msg.extend_from_slice(b"Sign to execute a burn-mint:\n");
                msg.extend_from_slice(b"0x"); msg.extend_from_slice(hex::encode(&self.data).as_bytes());
                msg
            }
            _ => vec![],
        }
    }

    pub fn assert_mint_opposite_side(&self) -> ProgramResult {
        if self.data[16] != Constants::HUB_ID {
            Err(FreeTunnelError::NotMintOppositeSide.into())
        } else { Ok(()) }
    }

    pub fn assert_mint_side(&self) -> ProgramResult {
        if self.data[17] != Constants::HUB_ID {
            Err(FreeTunnelError::NotMintSide.into())
        } else { Ok(()) }
    }
}
