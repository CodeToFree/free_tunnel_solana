use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    constants::{Constants, EthAddress},
    core::permissions::Permissions,
    state::BasicStorage,
    utils::DataAccountUtils,
};

pub struct Processor;

impl Processor {
    fn process_initialize_executors<'a>(
        program_id: &Pubkey,
        signer_account: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_executors_at_index: &AccountInfo<'a>,
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

        // Process
        Permissions::init_executors_internal(
            data_account_basic_storage,
            data_account_executors_at_index,
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
}
