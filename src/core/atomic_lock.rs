use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::instruction::transfer;
use std::{fs::Permissions, mem::size_of};

use crate::{
    constants::{Constants, EthAddress},
    core::req_helpers::ReqId,
    error::FreeTunnelError,
    state::{BasicStorage, ExecutorsInfo, ProposedLock, TokensAndProposers},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct AtomicLock;

impl AtomicLock {
    fn propose_lock_internal<'a>(
        program_id: &Pubkey,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        system_account_token_program: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>, // signer
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        req_id.assert_from_chain_only()?;
        req_id.checked_created_time()?;
        if req_id.action() != 1 {
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

    fn execute_lock_internal<'a>(
        program_id: &Pubkey,
        payer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        system_account_token_program: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
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
        let mut token_and_proposers = DataAccountUtils::read_account_data::<TokensAndProposers>(
            data_account_tokens_proposers,
        )?;
        token_and_proposers.locked_balance[token_index as usize] += amount;
        DataAccountUtils::write_account_data(data_account_tokens_proposers, token_and_proposers)
    }
}
