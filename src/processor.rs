use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    constants::{Constants, EthAddress},
    core::{
        atomic_lock::AtomicLock, atomic_mint::AtomicMint, permissions::Permissions,
        req_helpers::ReqId,
    },
    error::FreeTunnelError,
    state::{BasicStorage, TokensAndProposers},
    utils::DataAccountUtils,
};

pub struct Processor;

impl Processor {
    fn check_is_mint_contract<'a>(
        data_account_basic_storage: &AccountInfo<'a>,
    ) -> ProgramResult {
        let basic_storage: BasicStorage = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        match basic_storage.mint_or_lock {
            true => Ok(()),
            false => Err(FreeTunnelError::NotMintContract.into()),
        }
    }

    fn check_is_lock_contract<'a>(
        data_account_basic_storage: &AccountInfo<'a>,
    ) -> ProgramResult {
        let basic_storage: BasicStorage = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        match basic_storage.mint_or_lock {
            true => Err(FreeTunnelError::NotLockContract.into()),
            false => Ok(()),
        }
    }

    fn process_initialize<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_executors_at_index: &AccountInfo<'a>,
        is_mint_contract: bool,
        executors: &Vec<EthAddress>,
        threshold: u64,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account
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
            data_account_executors_at_index,
            Constants::PREFIX_EXECUTORS,
            &exe_index.to_le_bytes(),
        )?;


        // Check signer
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Create data accounts and write
        DataAccountUtils::create_related_account(
            program_id,
            account_payer,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
            Constants::SIZE_BASIC_STORAGE,
        )?;
        DataAccountUtils::write_account_data(data_account_basic_storage, BasicStorage {
            mint_or_lock: is_mint_contract,
            admin: *account_admin.key,
            executors_group_length: 0,
        })?;
        DataAccountUtils::create_related_account(
            program_id,
            account_payer,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
            Constants::SIZE_TOKENS_PROPOSERS,
        )?;
        DataAccountUtils::write_account_data(data_account_tokens_proposers, TokensAndProposers {
            tokens: [Pubkey::default(); 256],
            decimals: [0; 256],
            locked_balance: [0; 256],
            proposers: Vec::new(),
        })?;

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
        account_admin: &AccountInfo<'a>,
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
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Update storage
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        basic_storage.admin = *new_admin;
        DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)
    }

    fn process_add_proposer<'a>(
        program_id: &Pubkey,
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
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
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        Permissions::add_proposer_internal(data_account_tokens_proposers, new_proposer)
    }

    fn process_remove_proposer<'a>(
        program_id: &Pubkey,
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
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
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        Permissions::remove_proposer_internal(data_account_tokens_proposers, proposer)
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
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
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
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        let mut token_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        if token_proposers.tokens[token_index as usize] != Pubkey::default() {
            Err(FreeTunnelError::TokenIndexOccupied.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else {
            token_proposers.tokens[token_index as usize] = *token_pubkey;
            DataAccountUtils::write_account_data(data_account_tokens_proposers, token_proposers)
        }
    }

    fn process_remove_token<'a>(
        program_id: &Pubkey,
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
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
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        let mut token_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        if token_proposers.tokens[token_index as usize] == Pubkey::default() {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else {
            token_proposers.tokens[token_index as usize] = Pubkey::default();
            DataAccountUtils::write_account_data(data_account_tokens_proposers, token_proposers)
        }
    }

    fn process_propose_mint<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
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
            data_account_proposed_mint,
            Constants::PREFIX_MINT,
            &req_id.data,
        )?;
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }
        AtomicMint::check_propose_mint(
            data_account_tokens_proposers,
            account_proposer.key,
            req_id,
        )?;

        // Process
        AtomicMint::propose_mint_internal(
            program_id,
            account_payer,
            data_account_tokens_proposers,
            data_account_proposed_mint,
            req_id,
            recipient,
        )
    }


    fn process_propose_mint_for_burn<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
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
            data_account_proposed_mint,
            Constants::PREFIX_MINT,
            &req_id.data,
        )?;
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }
        AtomicMint::check_propose_mint_from_burn(
            data_account_tokens_proposers,
            account_proposer.key,
            req_id,
        )?;

        // Process
        AtomicMint::propose_mint_internal(
            program_id,
            account_payer,
            data_account_tokens_proposers,
            data_account_proposed_mint,
            req_id,
            recipient,
        )
    }

    fn process_execute_mint<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        token_account_recipient: &AccountInfo<'a>,
        account_token_mint: &AccountInfo<'a>,
        account_multisig_owner: &AccountInfo<'a>,
        account_multisig_wallets: &[AccountInfo<'a>],
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
            data_account_proposed_mint,
            Constants::PREFIX_MINT,
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
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Process
        AtomicMint::execute_mint_internal(
            program_id,
            system_account_token_program,
            data_account_basic_storage,
            data_account_tokens_proposers,
            data_account_proposed_mint,
            data_account_current_executors,
            data_account_next_executors,
            token_account_recipient,
            account_token_mint,
            account_multisig_owner,
            account_multisig_wallets,
            req_id,
            signatures,
            executors,
            exe_index,
        )
    }

    fn process_cancel_mint<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match(
            program_id,
            data_account_proposed_mint,
            Constants::PREFIX_MINT,
            &req_id.data,
        )?;
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Process
        AtomicMint::cancel_mint_internal(
            program_id,
            data_account_proposed_mint,
            req_id,
        )
    }

    fn process_propose_burn<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
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
            data_account_proposed_burn,
            Constants::PREFIX_BURN,
            &req_id.data,
        )?;
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }
        AtomicMint::check_propose_burn(req_id)?;

        // Process
        AtomicMint::propose_burn_internal(
            program_id,
            system_account_token_program,
            account_payer,
            data_account_tokens_proposers,
            data_account_proposed_burn,
            token_account_proposer,
            token_account_contract,
            account_proposer,
            req_id,
        )
    }

    fn process_propose_burn_for_mint<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
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
            data_account_proposed_burn,
            Constants::PREFIX_BURN,
            &req_id.data,
        )?;
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }
        AtomicMint::check_propose_burn_from_mint(req_id)?;
        
        // Process
        AtomicMint::propose_burn_internal(
            program_id,
            system_account_token_program,
            account_payer,
            data_account_tokens_proposers,
            data_account_proposed_burn,
            token_account_proposer,
            token_account_contract,
            account_proposer,
            req_id,
        )
    }

    fn process_execute_burn<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        token_account_contract: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        account_token_mint: &AccountInfo<'a>,
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
            data_account_proposed_burn,
            Constants::PREFIX_BURN,
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
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Process
        AtomicMint::execute_burn_internal(
            program_id,
            system_account_token_program,
            data_account_basic_storage,
            data_account_tokens_proposers,
            data_account_proposed_burn,
            data_account_current_executors,
            data_account_next_executors,
            token_account_contract,
            account_contract_signer,
            account_token_mint,
            req_id,
            signatures,
            executors,
            exe_index,
        )
    }

    fn process_cancel_burn<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
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
            data_account_proposed_burn,
            Constants::PREFIX_BURN,
            &req_id.data,
        )?;
        Self::check_is_mint_contract(data_account_basic_storage)?;

        // Process
        AtomicMint::cancel_burn_internal(
            program_id,
            system_account_token_program,
            data_account_tokens_proposers,
            data_account_proposed_burn,
            token_account_proposer,
            token_account_contract,
            account_contract_signer,
            req_id,
        )
    }

    fn process_propose_lock<'a>(
        program_id: &Pubkey,
        account_payer: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
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
        Self::check_is_lock_contract(data_account_basic_storage)?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }

        // Process
        AtomicLock::propose_lock_internal(
            program_id,
            system_account_token_program,
            account_payer,
            data_account_tokens_proposers,
            data_account_proposed_lock,
            token_account_proposer,
            token_account_contract,
            account_proposer,
            req_id,
        )
    }

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
        Self::check_is_lock_contract(data_account_basic_storage)?;

        // Process
        AtomicLock::execute_lock_internal(
            program_id,
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
        data_account_basic_storage: &AccountInfo<'a>,
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
        Self::check_is_lock_contract(data_account_basic_storage)?;

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
        account_payer: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
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
        Self::check_is_lock_contract(data_account_basic_storage)?;

        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }

        // Process
        AtomicLock::propose_unlock_internal(
            program_id,
            account_payer,
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
        token_account_recipient: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
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
        Self::check_is_lock_contract(data_account_basic_storage)?;

        // Process
        AtomicLock::execute_unlock_internal(
            program_id,
            system_account_token_program,
            data_account_basic_storage,
            data_account_tokens_proposers,
            data_account_proposed_unlock,
            data_account_current_executors,
            data_account_next_executors,
            token_account_recipient,
            token_account_contract,
            account_contract_signer,
            req_id,
            signatures,
            executors,
            exe_index,
        )
    }

    fn process_cancel_unlock<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
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
        Self::check_is_lock_contract(data_account_basic_storage)?;

        // Process
        AtomicLock::cancel_unlock_internal(
            program_id,
            data_account_tokens_proposers,
            data_account_proposed_unlock,
            req_id,
        )
    }
}
