use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    constants::{Constants, EthAddress}, error::FreeTunnelError, instruction::FreeTunnelInstruction, logic::{
        atomic_lock::AtomicLock, atomic_mint::AtomicMint, permissions::Permissions,
        req_helpers::ReqId,
    }, state::{BasicStorage, SparseArray, TokensAndProposers}, utils::DataAccountUtils
};

pub struct Processor;

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = FreeTunnelInstruction::unpack(instruction_data)?;
        let accounts_iter = &mut accounts.iter();

        match instruction {
            FreeTunnelInstruction::Initialize {
                is_mint_contract,
                executors,
                threshold,
                exe_index,
            } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_executors_at_index = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_initialize(
                    program_id,
                    account_payer,
                    account_admin,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_executors_at_index,
                    system_program,
                    is_mint_contract,
                    &executors,
                    threshold,
                    exe_index,
                )
            }
            FreeTunnelInstruction::TransferAdmin { new_admin } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                Self::process_transfer_admin(
                    program_id,
                    account_admin,
                    data_account_basic_storage,
                    &new_admin,
                )
            }
            FreeTunnelInstruction::AddProposer { new_proposer } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                Self::process_add_proposer(
                    program_id,
                    account_admin,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    &new_proposer,
                )
            }
            FreeTunnelInstruction::RemoveProposer { proposer } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                Self::process_remove_proposer(
                    program_id,
                    account_admin,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    &proposer,
                )
            }
            FreeTunnelInstruction::UpdateExecutors {
                new_executors,
                threshold,
                active_since,
                signatures,
                executors,
                exe_index,
            } => {
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_current_executors = next_account_info(accounts_iter)?;
                let data_account_next_executors = next_account_info(accounts_iter)?;
                Self::process_update_executors(
                    program_id,
                    data_account_basic_storage,
                    data_account_current_executors,
                    data_account_next_executors,
                    &new_executors,
                    threshold,
                    active_since,
                    &signatures,
                    &executors,
                    exe_index,
                )
            }
            FreeTunnelInstruction::AddToken {
                token_index,
                token_pubkey,
                token_decimals,
            } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                Self::process_add_token(
                    program_id,
                    account_admin,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    token_index,
                    &token_pubkey,
                    token_decimals,
                )
            }
            FreeTunnelInstruction::RemoveToken { token_index } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                Self::process_remove_token(
                    program_id,
                    account_admin,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    token_index,
                )
            }
            FreeTunnelInstruction::ProposeMint { req_id, recipient } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_propose_mint(
                    program_id,
                    account_payer,
                    account_proposer,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_mint,
                    system_program,
                    &req_id,
                    &recipient,
                )
            }
            FreeTunnelInstruction::ProposeMintForBurn { req_id, recipient } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_propose_mint_for_burn(
                    program_id,
                    account_payer,
                    account_proposer,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_mint,
                    system_program,
                    &req_id,
                    &recipient,
                )
            }
            FreeTunnelInstruction::ExecuteMint {
                req_id,
                signatures,
                executors,
                exe_index,
            } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                let data_account_current_executors = next_account_info(accounts_iter)?;
                let data_account_next_executors = next_account_info(accounts_iter)?;
                let token_account_recipient = next_account_info(accounts_iter)?;
                let account_token_mint = next_account_info(accounts_iter)?;
                let account_multisig_owner = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                Self::process_execute_mint(
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
                    account_contract_signer,
                    &req_id,
                    &signatures,
                    &executors,
                    exe_index,
                )
            }
            FreeTunnelInstruction::CancelMint { req_id } => {
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                Self::process_cancel_mint(
                    program_id,
                    data_account_basic_storage,
                    data_account_proposed_mint,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeBurn { req_id } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_propose_burn(
                    program_id,
                    account_payer,
                    account_proposer,
                    system_account_token_program,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_burn,
                    token_account_proposer,
                    token_account_contract,
                    system_program,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeBurnForMint { req_id } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_propose_burn_for_mint(
                    program_id,
                    account_payer,
                    account_proposer,
                    system_account_token_program,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_burn,
                    token_account_proposer,
                    token_account_contract,
                    system_program,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ExecuteBurn {
                req_id,
                signatures,
                executors,
                exe_index,
            } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                let data_account_current_executors = next_account_info(accounts_iter)?;
                let data_account_next_executors = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                let account_token_mint = next_account_info(accounts_iter)?;
                Self::process_execute_burn(
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
                    &req_id,
                    &signatures,
                    &executors,
                    exe_index,
                )
            }
            FreeTunnelInstruction::CancelBurn { req_id } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                Self::process_cancel_burn(
                    program_id,
                    system_account_token_program,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_burn,
                    token_account_proposer,
                    token_account_contract,
                    account_contract_signer,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeLock { req_id } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_lock = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_propose_lock(
                    program_id,
                    account_payer,
                    account_proposer,
                    system_account_token_program,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_lock,
                    token_account_proposer,
                    token_account_contract,
                    system_program,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ExecuteLock {
                req_id,
                signatures,
                executors,
                exe_index,
            } => {
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_lock = next_account_info(accounts_iter)?;
                let data_account_current_executors = next_account_info(accounts_iter)?;
                let data_account_next_executors = next_account_info(accounts_iter)?;
                Self::process_execute_lock(
                    program_id,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_lock,
                    data_account_current_executors,
                    data_account_next_executors,
                    &req_id,
                    &signatures,
                    &executors,
                    exe_index,
                )
            }
            FreeTunnelInstruction::CancelLock { req_id } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_lock = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                Self::process_cancel_lock(
                    program_id,
                    system_account_token_program,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_lock,
                    token_account_proposer,
                    token_account_contract,
                    account_contract_signer,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeUnlock { req_id, recipient } => {
                let account_payer = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_unlock = next_account_info(accounts_iter)?;
                let system_program = next_account_info(accounts_iter)?;
                Self::process_propose_unlock(
                    program_id,
                    account_payer,
                    account_proposer,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_unlock,
                    system_program,
                    &req_id,
                    &recipient,
                )
            }
            FreeTunnelInstruction::ExecuteUnlock {
                req_id,
                signatures,
                executors,
                exe_index,
            } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_unlock = next_account_info(accounts_iter)?;
                let data_account_current_executors = next_account_info(accounts_iter)?;
                let data_account_next_executors = next_account_info(accounts_iter)?;
                let token_account_recipient = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                Self::process_execute_unlock(
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
                    &req_id,
                    &signatures,
                    &executors,
                    exe_index,
                )
            }
            FreeTunnelInstruction::CancelUnlock { req_id } => {
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_tokens_proposers = next_account_info(accounts_iter)?;
                let data_account_proposed_unlock = next_account_info(accounts_iter)?;
                Self::process_cancel_unlock(
                    program_id,
                    data_account_basic_storage,
                    data_account_tokens_proposers,
                    data_account_proposed_unlock,
                    &req_id,
                )
            }
        }
    }

    fn check_is_mint_contract<'a>(data_account_basic_storage: &AccountInfo<'a>) -> ProgramResult {
        let basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        match basic_storage.mint_or_lock {
            true => Ok(()),
            false => Err(FreeTunnelError::NotMintContract.into()),
        }
    }

    fn check_is_lock_contract<'a>(data_account_basic_storage: &AccountInfo<'a>) -> ProgramResult {
        let basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
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
        system_program: &AccountInfo<'a>,
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
        DataAccountUtils::create_data_account(
            program_id,
            system_program,
            account_payer,
            data_account_basic_storage,
            Constants::BASIC_STORAGE,
            b"",
            Constants::SIZE_BASIC_STORAGE + Constants::SIZE_LENGTH,
            BasicStorage {
                mint_or_lock: is_mint_contract,
                admin: *account_admin.key,
                executors_group_length: 0,
            },
        )?;
        DataAccountUtils::create_data_account(
            program_id,
            system_program,
            account_payer,
            data_account_tokens_proposers,
            Constants::TOKENS_PROPOSERS,
            b"",
            Constants::SIZE_TOKENS_PROPOSERS + Constants::SIZE_LENGTH,
            TokensAndProposers {
                tokens: SparseArray::default(),
                decimals: SparseArray::default(),
                locked_balance: SparseArray::default(),
                proposers: Vec::new(),
            },
        )?;

        Permissions::init_executors_internal(
            program_id,
            system_program,
            account_payer,
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
        token_decimals: u8,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_basic_storage, data_account_tokens_proposers],
            &[Constants::BASIC_STORAGE, Constants::TOKENS_PROPOSERS],
            &[b"", b""],
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        let mut token_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        if token_proposers.tokens.get(token_index) != Option::None {
            Err(FreeTunnelError::TokenIndexOccupied.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else {
            token_proposers.tokens.insert(token_index, *token_pubkey);
            token_proposers.decimals.insert(token_index, token_decimals);
            token_proposers.locked_balance.insert(token_index, 0);
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
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_basic_storage, data_account_tokens_proposers],
            &[Constants::BASIC_STORAGE, Constants::TOKENS_PROPOSERS],
            &[b"", b""],
        )?;

        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin.key)?;
        if !account_admin.is_signer {
            return Err(FreeTunnelError::AdminNotSigner.into());
        }

        // Process
        let mut token_proposers: TokensAndProposers =
            DataAccountUtils::read_account_data(data_account_tokens_proposers)?;
        if token_proposers.tokens.get(token_index) == Option::None {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else if token_proposers.locked_balance[token_index] != 0 {
            Err(FreeTunnelError::TokenStillInUse.into())
        } else {
            token_proposers.tokens.remove(token_index);
            token_proposers.decimals.remove(token_index);
            token_proposers.locked_balance.remove(token_index);
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
        system_program: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_mint],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_MINT],
            &[b"", &req_id.data],
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
            system_program,
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
        system_program: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_mint],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_MINT],
            &[b"", &req_id.data],
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
            system_program,
            req_id,
            recipient,
        )
    }

    fn process_execute_mint<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo<'a>,
        data_account_next_executors: &AccountInfo<'a>,
        token_account_recipient: &AccountInfo<'a>,
        account_token_mint: &AccountInfo<'a>,
        account_multisig_owner: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[
                data_account_basic_storage,
                data_account_tokens_proposers,
                data_account_proposed_mint,
                data_account_current_executors,
                data_account_next_executors,
            ],
            &[
                Constants::BASIC_STORAGE,
                Constants::TOKENS_PROPOSERS,
                Constants::PREFIX_MINT,
                Constants::PREFIX_EXECUTORS,
                Constants::PREFIX_EXECUTORS,
            ],
            &[
                b"",
                b"",
                &req_id.data,
                &exe_index.to_le_bytes(),
                &(exe_index + 1).to_le_bytes(),
            ],
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
            account_contract_signer,
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
        AtomicMint::cancel_mint_internal(program_id, data_account_proposed_mint, req_id)
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
        system_program: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_burn],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_BURN],
            &[b"", &req_id.data],
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
            system_program,
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
        system_program: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_burn],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_BURN],
            &[b"", &req_id.data],
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
            system_program,
            req_id,
        )
    }

    fn process_execute_burn<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo<'a>,
        data_account_next_executors: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        account_token_mint: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[
                data_account_basic_storage,
                data_account_tokens_proposers,
                data_account_proposed_burn,
                data_account_current_executors,
                data_account_next_executors,
            ],
            &[
                Constants::BASIC_STORAGE,
                Constants::TOKENS_PROPOSERS,
                Constants::PREFIX_BURN,
                Constants::PREFIX_EXECUTORS,
                Constants::PREFIX_EXECUTORS,
            ],
            &[
                b"",
                b"",
                &req_id.data,
                &exe_index.to_le_bytes(),
                &(exe_index + 1).to_le_bytes(),
            ],
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
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_burn],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_BURN],
            &[b"", &req_id.data],
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
        system_program: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_lock],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_LOCK],
            &[b"", &req_id.data],
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
            system_program,
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
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[
                data_account_basic_storage,
                data_account_tokens_proposers,
                data_account_proposed_lock,
                data_account_current_executors,
                data_account_next_executors,
            ],
            &[
                Constants::BASIC_STORAGE,
                Constants::TOKENS_PROPOSERS,
                Constants::PREFIX_LOCK,
                Constants::PREFIX_EXECUTORS,
                Constants::PREFIX_EXECUTORS,
            ],
            &[
                b"",
                b"",
                &req_id.data,
                &exe_index.to_le_bytes(),
                &(exe_index + 1).to_le_bytes(),
            ],
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
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[
                data_account_tokens_proposers,
                data_account_proposed_lock,
                account_contract_signer,
            ],
            &[
                Constants::TOKENS_PROPOSERS,
                Constants::PREFIX_LOCK,
                Constants::CONTRACT_SIGNER,
            ],
            &[b"", &req_id.data, b""],
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
        system_program: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_unlock],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_UNLOCK],
            &[b"", &req_id.data],
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
            system_program,
            req_id,
            recipient,
        )
    }

    fn process_execute_unlock<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        data_account_current_executors: &AccountInfo<'a>,
        data_account_next_executors: &AccountInfo<'a>,
        token_account_recipient: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        // Check data account conditions
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[
                data_account_basic_storage,
                data_account_tokens_proposers,
                data_account_proposed_unlock,
                data_account_current_executors,
                data_account_next_executors,
                account_contract_signer,
            ],
            &[
                Constants::BASIC_STORAGE,
                Constants::TOKENS_PROPOSERS,
                Constants::PREFIX_UNLOCK,
                Constants::PREFIX_EXECUTORS,
                Constants::PREFIX_EXECUTORS,
                Constants::CONTRACT_SIGNER,
            ],
            &[
                b"",
                b"",
                &req_id.data,
                &exe_index.to_le_bytes(),
                &(exe_index + 1).to_le_bytes(),
                b"",
            ],
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
        DataAccountUtils::check_account_match_batch(
            program_id,
            &[data_account_tokens_proposers, data_account_proposed_unlock],
            &[Constants::TOKENS_PROPOSERS, Constants::PREFIX_UNLOCK],
            &[b"", &req_id.data],
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
