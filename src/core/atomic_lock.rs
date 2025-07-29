use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, program::invoke_signed,
    pubkey::Pubkey, sysvar::Sysvar,
};
use spl_token::instruction::transfer;
use std::mem::size_of;

use crate::{
    constants::{Constants, EthAddress},
    core::{permissions::Permissions, req_helpers::ReqId},
    error::FreeTunnelError,
    state::{ProposedLock, TokensAndProposers},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct AtomicLock;

impl AtomicLock {
    pub(crate) fn propose_lock_internal<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>, // signer
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        req_id.assert_from_chain_only()?;
        req_id.checked_created_time()?;
        if req_id.action() & 0x0f != 1 {
            return Err(FreeTunnelError::NotLockMint.into());
        }
        if !data_account_proposed_lock.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Write proposed-lock data
        DataAccountUtils::create_related_account(
            program_id,
            payer_account,
            data_account_proposed_lock,
            Constants::PREFIX_LOCK,
            &req_id.data,
            size_of::<ProposedLock>() + Constants::SIZE_LENGTH,
        )?;
        DataAccountUtils::write_account_data(
            data_account_proposed_lock,
            ProposedLock {
                inner: *account_proposer.key,
            },
        )?;

        // Deposit token
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        invoke_signed(
            &transfer(
                system_account_token_program.key,
                token_account_proposer.key,
                token_account_contract.key,
                account_proposer.key,
                &[],
                amount,
            )?,
            &[
                token_account_proposer.clone(),
                token_account_contract.clone(),
                account_proposer.clone(),
            ],
            &[],
        )
    }

    pub(crate) fn execute_lock_internal<'a>(
        data_account_basic_storage: &AccountInfo,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check conditions
        let proposer =
            DataAccountUtils::read_account_data::<ProposedLock>(data_account_proposed_lock)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Check signatures
        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::check_multi_signatures(
            data_account_basic_storage,
            data_account_current_executors,
            data_account_next_executors,
            &message,
            signatures,
            executors,
            exe_index,
        )?;

        // Update proposed-lock data
        DataAccountUtils::write_account_data(
            data_account_proposed_lock,
            ProposedLock {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Update locked-balance data
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let token_index = req_id.checked_token_index(data_account_tokens_proposers)?;
        let mut token_and_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        token_and_proposers.locked_balance[token_index as usize] += amount;
        DataAccountUtils::write_account_data(data_account_tokens_proposers, token_and_proposers)
    }

    pub(crate) fn cancel_lock_internal<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        let proposer =
            DataAccountUtils::read_account_data::<ProposedLock>(data_account_proposed_lock)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        let now = Clock::get()?.unix_timestamp;
        if now < (req_id.created_time() + Constants::EXPIRE_PERIOD) as i64 {
            return Err(FreeTunnelError::WaitUntilExpired.into());
        }

        // Update proposed-lock data
        DataAccountUtils::write_account_data(
            data_account_proposed_lock,
            ProposedLock {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Refund token
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let (expected_contract_pubkey, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if expected_contract_pubkey != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &transfer(
                system_account_token_program.key,
                token_account_contract.key,
                token_account_proposer.key,
                account_contract_signer.key,
                &[],
                amount,
            )?,
            &[
                token_account_contract.clone(),
                token_account_proposer.clone(),
                account_contract_signer.clone(),
            ],
            &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]],
        )
    }

    pub(crate) fn propose_unlock_internal<'a>(
        program_id: &Pubkey,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>, // signer
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check conditions
        Permissions::assert_only_proposer(data_account_tokens_proposers, account_proposer.key)?;
        req_id.assert_from_chain_only()?;
        req_id.checked_created_time()?;
        if req_id.action() & 0x0f != 2 {
            return Err(FreeTunnelError::NotBurnUnlock.into());
        }
        if !data_account_proposed_unlock.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        if *recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidRecipient.into());
        }

        // Write proposed-unlock data
        DataAccountUtils::create_related_account(
            program_id,
            payer_account,
            data_account_proposed_unlock,
            Constants::PREFIX_UNLOCK,
            &req_id.data,
            size_of::<ProposedLock>() + Constants::SIZE_LENGTH,
        )?;
        DataAccountUtils::write_account_data(
            data_account_proposed_unlock,
            ProposedLock { inner: *recipient },
        )?;

        // Update locked-balance data
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let token_index = req_id.checked_token_index(data_account_tokens_proposers)?;
        let mut token_and_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        token_and_proposers.locked_balance[token_index as usize] -= amount;
        DataAccountUtils::write_account_data(data_account_tokens_proposers, token_and_proposers)
    }

    pub(crate) fn execute_unlock_internal<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        token_account_contract: &AccountInfo<'a>,
        token_account_recipient: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check conditions
        let recipient =
            DataAccountUtils::read_account_data::<ProposedLock>(data_account_proposed_unlock)?
                .inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Check signatures
        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::check_multi_signatures(
            data_account_basic_storage,
            data_account_current_executors,
            data_account_next_executors,
            &message,
            signatures,
            executors,
            exe_index,
        )?;

        // Update proposed-unlock data
        DataAccountUtils::write_account_data(
            data_account_proposed_unlock,
            ProposedLock {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Unlock token to recipient
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let (expected_contract_pubkey, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if expected_contract_pubkey != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &transfer(
                system_account_token_program.key,
                token_account_contract.key,
                token_account_recipient.key,
                account_contract_signer.key,
                &[],
                amount,
            )?,
            &[
                token_account_contract.clone(),
                token_account_recipient.clone(),
                account_contract_signer.clone(),
            ],
            &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]],
        )
    }

    pub(crate) fn cancel_unlock_internal<'a>(
        _program_id: &Pubkey,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        let recipient =
            DataAccountUtils::read_account_data::<ProposedLock>(data_account_proposed_unlock)?
                .inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        let now = Clock::get()?.unix_timestamp;
        if now < (req_id.created_time() + Constants::EXPIRE_EXTRA_PERIOD) as i64 {
            return Err(FreeTunnelError::WaitUntilExpired.into());
        }

        // Update proposed-unlock data
        DataAccountUtils::write_account_data(
            data_account_proposed_unlock,
            ProposedLock {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Update locked-balance data
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let token_index = req_id.checked_token_index(data_account_tokens_proposers)?;
        let mut token_and_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        token_and_proposers.locked_balance[token_index as usize] += amount;
        DataAccountUtils::write_account_data(data_account_tokens_proposers, token_and_proposers)
    }
}
