use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    constants::{Constants, EthAddress},
    core::{atomic_lock::AtomicLock, permissions::Permissions, req_helpers::ReqId},
    error::FreeTunnelError,
    state::{BasicStorage, TokensAndProposers},
    utils::DataAccountUtils,
};

pub struct Processor;

impl Processor {
    fn process_initialize_executors<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_executors_at_index: &AccountInfo<'a>,
        account_admin: &AccountInfo<'a>,
        executors: &Vec<EthAddress>,
        threshold: u64,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_executors_at_index,
            Constants::PREFIX_EXECUTORS,
            &exe_index.to_le_bytes(),
        )?;

        // Check signer
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        Permissions::init_executors_internal(
            data_account_basic_storage,
            data_account_executors_at_index,
            account_admin.key,
            executors,
            threshold,
            exe_index,
        )
    }

    fn process_transfer_admin<'a>(
        program_id: &Pubkey,
        signer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        new_admin: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, signer_account.key)?;

        // Update storage
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        basic_storage.admin = *new_admin;
        DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)
    }

    fn process_add_proposer<'a>(
        program_id: &Pubkey,
        signer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_token_proposers: &AccountInfo<'a>,
        new_proposer: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_token_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, signer_account.key)?;

        // Process
        Permissions::add_proposer_internal(data_account_token_proposers, new_proposer)
    }

    fn process_remove_proposer<'a>(
        program_id: &Pubkey,
        signer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_token_proposers: &AccountInfo<'a>,
        proposer: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_token_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, signer_account.key)?;

        // Process
        Permissions::remove_proposer_internal(data_account_token_proposers, proposer)
    }

    fn process_update_executors<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo<'a>,
        data_account_next_executors: &AccountInfo<'a>,
        new_executors: &Vec<EthAddress>,
        threshold: u64,
        active_since: u64,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_current_executors,
            Constants::PREFIX_EXECUTORS,
            &exe_index.to_le_bytes(),
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_next_executors,
            Constants::PREFIX_EXECUTORS,
            &(exe_index + 1).to_le_bytes(),
        )?;

        // Process
        Permissions::update_executors(
            data_account_basic_storage,
            data_account_current_executors,
            data_account_next_executors,
            new_executors,
            threshold,
            active_since,
            signatures,
            executors,
            exe_index,
        )
    }

    fn process_add_token<'a>(
        program_id: &Pubkey,
        signer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_token_proposers: &AccountInfo<'a>,
        token_index: u8,
        token_pubkey: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_token_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, signer_account.key)?;

        // Process
        let mut token_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_token_proposers)?;
        if token_proposers.tokens[token_index as usize] != Pubkey::default() {
            Err(FreeTunnelError::TokenIndexOccupied.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else {
            token_proposers.tokens[token_index as usize] = *token_pubkey;
            DataAccountUtils::write_account_data(data_account_token_proposers, token_proposers)
        }
    }

    fn process_remove_token<'a>(
        program_id: &Pubkey,
        signer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_token_proposers: &AccountInfo<'a>,
        token_index: u8,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_token_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, signer_account.key)?;

        // Process
        let mut token_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_token_proposers)?;
        if token_proposers.tokens[token_index as usize] == Pubkey::default() {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else {
            token_proposers.tokens[token_index as usize] = Pubkey::default();
            DataAccountUtils::write_account_data(data_account_token_proposers, token_proposers)
        }
    }

    fn process_propose_lock<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_lock,
            Constants::PREFIX_LOCK,
            &req_id.data,
        )?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }

        // Process
        AtomicLock::propose_lock_internal(
            program_id,
            system_account_token_program,
            payer_account,
            data_account_tokens_proposers,
            data_account_proposed_lock,
            token_account_proposer,
            token_account_contract,
            account_proposer,
            req_id,
        )
    }

    // pub(crate) fn execute_lock_internal<'a>(
    //     data_account_basic_storage: &AccountInfo,
    //     data_account_tokens_proposers: &AccountInfo<'a>,
    //     data_account_proposed_lock: &AccountInfo<'a>,
    //     data_account_current_executors: &AccountInfo,
    //     data_account_next_executors: &AccountInfo,
    //     req_id: &ReqId,
    //     signatures: &Vec<[u8; 64]>,
    //     executors: &Vec<EthAddress>,
    //     exe_index: u64,
    // ) -> ProgramResult {

    fn process_execute_lock<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo<'a>,
        data_account_next_executors: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_lock,
            Constants::PREFIX_LOCK,
            &req_id.data,
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_current_executors,
            Constants::PREFIX_EXECUTORS,
            &exe_index.to_le_bytes(),
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_next_executors,
            Constants::PREFIX_EXECUTORS,
            &(exe_index + 1).to_le_bytes(),
        )?;

        // Process
        AtomicLock::execute_lock_internal(
            data_account_basic_storage,
            data_account_tokens_proposers,
            data_account_proposed_lock,
            data_account_current_executors,
            data_account_next_executors,
            req_id,
            signatures,
            executors,
            exe_index,
        )
    }

    fn process_cancel_lock<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_lock,
            Constants::PREFIX_LOCK,
            &req_id.data,
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            account_contract_signer,
            Constants::CONTRACT_SIGNER,
            b"",
        )?;

        // Process
        AtomicLock::cancel_lock_internal(
            program_id,
            system_account_token_program,
            data_account_tokens_proposers,
            data_account_proposed_lock,
            token_account_proposer,
            token_account_contract,
            account_contract_signer,
            req_id,
        )
    }

    fn process_propose_unlock<'a>(
        program_id: &Pubkey,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_unlock,
            Constants::PREFIX_UNLOCK,
            &req_id.data,
        )?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }

        // Process
        AtomicLock::propose_unlock_internal(
            program_id,
            payer_account,
            data_account_tokens_proposers,
            data_account_proposed_unlock,
            account_proposer,
            req_id,
            recipient,
        )
    }

    fn process_execute_unlock<'a>(
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
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_unlock,
            Constants::PREFIX_UNLOCK,
            &req_id.data,
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_current_executors,
            Constants::PREFIX_EXECUTORS,
            &exe_index.to_le_bytes(),
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_next_executors,
            Constants::PREFIX_EXECUTORS,
            &(exe_index + 1).to_le_bytes(),
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            account_contract_signer,
            Constants::CONTRACT_SIGNER,
            b"",
        )?;

        // Process
        AtomicLock::execute_unlock_internal(
            program_id,
            system_account_token_program,
            data_account_basic_storage,
            data_account_tokens_proposers,
            data_account_proposed_unlock,
            data_account_current_executors,
            data_account_next_executors,
            token_account_contract,
            token_account_recipient,
            account_contract_signer,
            req_id,
            signatures,
            executors,
            exe_index,
        )
    }

    fn process_cancel_unlock<'a>(
        program_id: &Pubkey,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_unlock,
            Constants::PREFIX_UNLOCK,
            &req_id.data,
        )?;

        // Process
        AtomicLock::cancel_unlock_internal(
            program_id,
            data_account_tokens_proposers,
            data_account_proposed_unlock,
            req_id,
        )
    }
}
