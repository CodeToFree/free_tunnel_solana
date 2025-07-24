use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

use crate::error::FreeTunnelError;
use crate::utils::DataAccountUtils;
use crate::{constants::Constants, state::TokensAndProposers};
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
        if ((time + Constants::PROPOSE_PERIOD) as i64) < now {
            Err(FreeTunnelError::CreatedTimeTooEarly.into())
        } else if (time as i64) > now + 60 {
            Err(FreeTunnelError::CreatedTimeTooLate.into())
        } else {
            Ok(time)
        }
    }

    pub fn action(&self) -> u8 {
        self.data[6]
    }

    pub fn token_index(&self) -> u8 {
        self.data[7]
    }

    pub fn checked_token_index(
        &self,
        data_account_tokens_proposers: &AccountInfo,
    ) -> Result<u8, ProgramError> {
        let TokensAndProposers { tokens, .. } =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        if tokens[self.token_index() as usize] == Pubkey::default() {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else {
            Ok(self.token_index())
        }
    }

    pub fn checked_token_pubkey_and_decimal(
        &self,
        data_account_tokens_proposers: &AccountInfo,
    ) -> Result<(Pubkey, u8), ProgramError> {
        let TokensAndProposers {
            tokens, decimals, ..
        } = DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        let token_pubkey = tokens[self.token_index() as usize];
        if token_pubkey == Pubkey::default() {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else {
            Ok((token_pubkey, decimals[self.token_index() as usize]))
        }
    }

    pub fn raw_amount(&self) -> u64 {
        u64::from_be_bytes(self.data[8..16].try_into().unwrap())
    }

    pub fn checked_amount(
        &self,
        data_account_tokens_proposers: &AccountInfo,
    ) -> Result<u64, ProgramError> {
        let amount = self.raw_amount();
        if amount == 0 {
            Err(FreeTunnelError::AmountCannotBeZero.into())
        } else {
            let (_, decimal) =
                self.checked_token_pubkey_and_decimal(data_account_tokens_proposers)?;
            if decimal > 6 {
                Ok(amount * 10u64.pow(decimal as u32 - 6))
            } else if decimal < 6 {
                Ok(amount / 10u64.pow(6 - decimal as u32))
            } else {
                Ok(amount)
            }
        }
    }

    pub fn msg_from_req_signing_message(&self) -> Vec<u8> {
        let specific_action = self.action() & 0x0f;
        let mut msg = Constants::ETH_SIGN_HEADER.to_vec();
        match specific_action {
            1 => {
                let length = 3 + Constants::BRIDGE_CHANNEL.len() + 29 + 66;
                msg.extend_from_slice(length.to_string().as_bytes());
                msg.extend_from_slice(b"[");
                msg.extend_from_slice(Constants::BRIDGE_CHANNEL);
                msg.extend_from_slice(b"]\n");
                msg.extend_from_slice(b"Sign to execute a lock-mint:\n");
                msg.extend_from_slice(b"0x");
                msg.extend_from_slice(hex::encode(&self.data).as_bytes());
                msg
            }
            2 => {
                let length = 3 + Constants::BRIDGE_CHANNEL.len() + 31 + 66;
                msg.extend_from_slice(length.to_string().as_bytes());
                msg.extend_from_slice(b"[");
                msg.extend_from_slice(Constants::BRIDGE_CHANNEL);
                msg.extend_from_slice(b"]\n");
                msg.extend_from_slice(b"Sign to execute a burn-unlock:\n");
                msg.extend_from_slice(b"0x");
                msg.extend_from_slice(hex::encode(&self.data).as_bytes());
                msg
            }
            3 => {
                let length = 3 + Constants::BRIDGE_CHANNEL.len() + 29 + 66;
                msg.extend_from_slice(length.to_string().as_bytes());
                msg.extend_from_slice(b"[");
                msg.extend_from_slice(Constants::BRIDGE_CHANNEL);
                msg.extend_from_slice(b"]\n");
                msg.extend_from_slice(b"Sign to execute a burn-mint:\n");
                msg.extend_from_slice(b"0x");
                msg.extend_from_slice(hex::encode(&self.data).as_bytes());
                msg
            }
            _ => vec![],
        }
    }

    pub fn assert_from_chain_only(&self) -> ProgramResult {
        if self.data[16] != Constants::CHAIN {
            Err(FreeTunnelError::NotFromCurrentChain.into())
        } else {
            Ok(())
        }
    }

    pub fn assert_to_chain_only(&self) -> ProgramResult {
        if self.data[17] != Constants::CHAIN {
            Err(FreeTunnelError::NotToCurrentChain.into())
        } else {
            Ok(())
        }
    }
}
