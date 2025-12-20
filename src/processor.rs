use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    system_program,
};

use spl_token::state::Mint;

use crate::{
    constants::Constants, error::FreeTunnelError, instruction::FreeTunnelInstruction, logic::{
        atomic_lock::AtomicLock, atomic_mint::AtomicMint, permissions::Permissions,
    }, state::{BasicStorage, SparseArray}, utils::DataAccountUtils
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
                let system_program = next_account_info(accounts_iter)?;
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_executors = next_account_info(accounts_iter)?;
                Self::assert_system_program(system_program)?;

                // Check data account
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_executors, Constants::PREFIX_EXECUTORS, &exe_index.to_le_bytes())?;

                // Create data accounts and write
                DataAccountUtils::create_data_account(
                    program_id,
                    system_program,
                    account_admin,
                    data_account_basic_storage,
                    Constants::BASIC_STORAGE,
                    b"",
                    Constants::SIZE_BASIC_STORAGE + Constants::SIZE_LENGTH,
                    BasicStorage {
                        mint_or_lock: is_mint_contract,
                        admin: *account_admin.key,
                        executors_group_length: 0,
                        tokens: SparseArray::default(),
                        decimals: SparseArray::default(),
                        locked_balance: SparseArray::default(),
                        proposers: Vec::new(),
                    },
                )?;

                // Process internal logic
                Permissions::init_executors(
                    program_id,
                    system_program,
                    account_admin,
                    data_account_basic_storage,
                    data_account_executors,
                    &executors,
                    threshold,
                    exe_index,
                )
            }
            FreeTunnelInstruction::TransferAdmin { new_admin } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                Self::process_transfer_admin(
                    account_admin,
                    data_account_basic_storage,
                    &new_admin,
                )
            }
            FreeTunnelInstruction::AddProposer { new_proposer } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                Permissions::add_proposer(account_admin, data_account_basic_storage, &new_proposer)
            }
            FreeTunnelInstruction::RemoveProposer { proposer } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                Permissions::remove_proposer(account_admin, data_account_basic_storage, &proposer)
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
                let data_account_executors = next_account_info(accounts_iter)?;
                let data_account_new_executors = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_executors, Constants::PREFIX_EXECUTORS, &exe_index.to_le_bytes())?;
                DataAccountUtils::check_account_match(program_id, data_account_new_executors, Constants::PREFIX_EXECUTORS, &(exe_index + 1).to_le_bytes())?;
                Permissions::update_executors(
                    data_account_basic_storage,
                    data_account_executors,
                    data_account_new_executors,
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
            } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let token_mint = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, &data_account_basic_storage, &Constants::BASIC_STORAGE, b"")?;
                Self::assert_token_mint_valid(token_mint)?;

                let mint_state = Mint::unpack(&token_mint.data.borrow())?;

                Self::process_add_token(
                    account_admin,
                    data_account_basic_storage,
                    token_index,
                    token_mint.key,
                    mint_state.decimals,
                )
            }
            FreeTunnelInstruction::RemoveToken { token_index } => {
                let account_admin = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, &Constants::BASIC_STORAGE, b"")?;
                Self::process_remove_token(
                    account_admin,
                    data_account_basic_storage,
                    token_index,
                )
            }
            FreeTunnelInstruction::ProposeMint { req_id, recipient } => {
                let system_program = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                Self::assert_system_program(system_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, &Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_mint, Constants::PREFIX_MINT, &req_id.data)?;
                AtomicMint::propose_mint(
                    program_id,
                    system_program,
                    account_proposer,
                    data_account_basic_storage,
                    data_account_proposed_mint,
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
                let account_contract_signer = next_account_info(accounts_iter)?;
                let token_account_recipient = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                let data_account_executors = next_account_info(accounts_iter)?;
                let token_mint = next_account_info(accounts_iter)?;
                let account_multisig_owner = next_account_info(accounts_iter)?;
                Self::assert_token_program(system_account_token_program)?;
                Self::assert_token_mint_valid(token_mint)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_mint, Constants::PREFIX_MINT, &req_id.data)?;
                DataAccountUtils::check_account_match(program_id, data_account_executors, Constants::PREFIX_EXECUTORS, &exe_index.to_le_bytes())?;
                DataAccountUtils::check_account_match(program_id, account_contract_signer, Constants::CONTRACT_SIGNER, b"")?;
                AtomicMint::execute_mint(
                    program_id,
                    system_account_token_program,
                    account_contract_signer,
                    token_account_recipient,
                    data_account_basic_storage,
                    data_account_proposed_mint,
                    data_account_executors,
                    token_mint,
                    account_multisig_owner,
                    &req_id,
                    &signatures,
                    &executors,
                )
            }
            FreeTunnelInstruction::CancelMint { req_id } => {
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_mint = next_account_info(accounts_iter)?;
                let account_refund = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_mint, Constants::PREFIX_MINT, &req_id.data)?;
                AtomicMint::cancel_mint(
                    program_id,
                    data_account_basic_storage,
                    data_account_proposed_mint,
                    account_refund,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeBurn { req_id } => {
                let system_program = next_account_info(accounts_iter)?;
                let system_account_token_program = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                Self::assert_system_program(system_program)?;
                Self::assert_token_program(system_account_token_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_burn, Constants::PREFIX_BURN, &req_id.data)?;
                AtomicMint::propose_burn(
                    program_id,
                    system_program,
                    system_account_token_program,
                    account_proposer,
                    token_account_contract,
                    token_account_proposer,
                    data_account_basic_storage,
                    data_account_proposed_burn,
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
                let account_contract_signer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                let data_account_executors = next_account_info(accounts_iter)?;
                let token_mint = next_account_info(accounts_iter)?;
                Self::assert_token_program(system_account_token_program)?;
                Self::assert_token_mint_valid(token_mint)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_burn, Constants::PREFIX_BURN, &req_id.data)?;
                DataAccountUtils::check_account_match(program_id, data_account_executors, Constants::PREFIX_EXECUTORS, &exe_index.to_le_bytes())?;
                DataAccountUtils::check_account_match(program_id, account_contract_signer, Constants::CONTRACT_SIGNER, b"")?;
                AtomicMint::execute_burn(
                    program_id,
                    system_account_token_program,
                    account_contract_signer,
                    token_account_contract,
                    data_account_basic_storage,
                    data_account_proposed_burn,
                    data_account_executors,
                    token_mint,
                    &req_id,
                    &signatures,
                    &executors,
                )
            }
            FreeTunnelInstruction::CancelBurn { req_id } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_burn = next_account_info(accounts_iter)?;
                let account_refund = next_account_info(accounts_iter)?;
                Self::assert_token_program(system_account_token_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_burn, Constants::PREFIX_BURN, &req_id.data)?;
                DataAccountUtils::check_account_match(program_id, account_contract_signer, Constants::CONTRACT_SIGNER, b"")?;
                AtomicMint::cancel_burn(
                    program_id,
                    system_account_token_program,
                    account_contract_signer,
                    token_account_contract,
                    token_account_proposer,
                    data_account_basic_storage,
                    data_account_proposed_burn,
                    account_refund,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeLock { req_id } => {
                let system_program = next_account_info(accounts_iter)?;
                let system_account_token_program = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_lock = next_account_info(accounts_iter)?;
                Self::assert_system_program(system_program)?;
                Self::assert_token_program(system_account_token_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_lock, Constants::PREFIX_LOCK, &req_id.data)?;
                AtomicLock::propose_lock(
                    program_id,
                    system_program,
                    system_account_token_program,
                    account_proposer,
                    token_account_contract,
                    token_account_proposer,
                    data_account_basic_storage,
                    data_account_proposed_lock,
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
                let data_account_proposed_lock = next_account_info(accounts_iter)?;
                let data_account_executors = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_lock, Constants::PREFIX_LOCK, &req_id.data)?;
                DataAccountUtils::check_account_match(program_id, data_account_executors, Constants::PREFIX_EXECUTORS, &exe_index.to_le_bytes())?;
                AtomicLock::execute_lock(
                    program_id,
                    data_account_basic_storage,
                    data_account_proposed_lock,
                    data_account_executors,
                    &req_id,
                    &signatures,
                    &executors,
                )
            }
            FreeTunnelInstruction::CancelLock { req_id } => {
                let system_account_token_program = next_account_info(accounts_iter)?;
                let account_contract_signer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let token_account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_lock = next_account_info(accounts_iter)?;
                let account_refund = next_account_info(accounts_iter)?;
                Self::assert_token_program(system_account_token_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, &Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_lock, Constants::PREFIX_LOCK, &req_id.data)?;
                DataAccountUtils::check_account_match(program_id, account_contract_signer, Constants::CONTRACT_SIGNER, b"")?;
                AtomicLock::cancel_lock(
                    program_id,
                    system_account_token_program,
                    account_contract_signer,
                    token_account_contract,
                    token_account_proposer,
                    data_account_basic_storage,
                    data_account_proposed_lock,
                    account_refund,
                    &req_id,
                )
            }
            FreeTunnelInstruction::ProposeUnlock { req_id, recipient } => {
                let system_program = next_account_info(accounts_iter)?;
                let account_proposer = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_unlock = next_account_info(accounts_iter)?;
                Self::assert_system_program(system_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_unlock, Constants::PREFIX_UNLOCK, &req_id.data)?;
                AtomicLock::propose_unlock(
                    program_id,
                    system_program,
                    account_proposer,
                    data_account_basic_storage,
                    data_account_proposed_unlock,
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
                let account_contract_signer = next_account_info(accounts_iter)?;
                let token_account_contract = next_account_info(accounts_iter)?;
                let token_account_recipient = next_account_info(accounts_iter)?;
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_unlock = next_account_info(accounts_iter)?;
                let data_account_executors = next_account_info(accounts_iter)?;
                Self::assert_token_program(system_account_token_program)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_unlock, Constants::PREFIX_UNLOCK, &req_id.data)?;
                DataAccountUtils::check_account_match(program_id, data_account_executors, Constants::PREFIX_EXECUTORS, &exe_index.to_le_bytes())?;
                DataAccountUtils::check_account_match(program_id, account_contract_signer, Constants::CONTRACT_SIGNER, b"")?;
                AtomicLock::execute_unlock(
                    program_id,
                    system_account_token_program,
                    account_contract_signer,
                    token_account_contract,
                    token_account_recipient,
                    data_account_basic_storage,
                    data_account_proposed_unlock,
                    data_account_executors,
                    &req_id,
                    &signatures,
                    &executors,
                )
            }
            FreeTunnelInstruction::CancelUnlock { req_id } => {
                let data_account_basic_storage = next_account_info(accounts_iter)?;
                let data_account_proposed_unlock = next_account_info(accounts_iter)?;
                let account_refund = next_account_info(accounts_iter)?;
                DataAccountUtils::check_account_match(program_id, data_account_basic_storage, Constants::BASIC_STORAGE, b"")?;
                DataAccountUtils::check_account_match(program_id, data_account_proposed_unlock, Constants::PREFIX_UNLOCK, &req_id.data)?;
                AtomicLock::cancel_unlock(
                    program_id,
                    data_account_basic_storage,
                    data_account_proposed_unlock,
                    account_refund,
                    &req_id,
                )
            }
        }
    }

    fn process_transfer_admin<'a>(
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        new_admin: &Pubkey,
    ) -> ProgramResult {
        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin)?;

        // Update storage
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        let prev_admin = basic_storage.admin;
        basic_storage.admin = *new_admin;
        DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)?;

        msg!(
            "AdminTransferred: prev_admin={}, new_admin={}",
            prev_admin,
            new_admin
        );
        Ok(())
    }

    fn process_add_token<'a>(
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        token_index: u8,
        token_pubkey: &Pubkey,
        token_decimals: u8,
    ) -> ProgramResult {
        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin)?;

        // Process
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if basic_storage.tokens.get(token_index) != Option::None {
            Err(FreeTunnelError::TokenIndexOccupied.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else {
            basic_storage.tokens.insert(token_index, *token_pubkey);
            basic_storage.decimals.insert(token_index, token_decimals);
            basic_storage.locked_balance.insert(token_index, 0);
            DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)?;

            msg!(
                "TokenAdded: token_index={}, token_mint={}, decimals={}",
                token_index,
                token_pubkey,
                token_decimals
            );
            Ok(())
        }
    }

    fn process_remove_token<'a>(
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        token_index: u8,
    ) -> ProgramResult {
        // Check permissions
        Permissions::assert_only_admin(data_account_basic_storage, account_admin)?;

        // Process
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if basic_storage.tokens.get(token_index) == Option::None {
            Err(FreeTunnelError::TokenIndexNonExistent.into())
        } else if token_index == 0 {
            Err(FreeTunnelError::TokenIndexCannotBeZero.into())
        } else if *basic_storage
            .locked_balance
            .get(token_index)
            .ok_or(FreeTunnelError::TokenIndexNonExistent)?
            != 0
        {
            Err(FreeTunnelError::LockedBalanceMustBeZero.into())
        } else {
            basic_storage.tokens.remove(token_index);
            basic_storage.decimals.remove(token_index);
            basic_storage.locked_balance.remove(token_index);
            DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)?;

            msg!("TokenRemoved: token_index={}", token_index);
            Ok(())
        }
    }

    fn assert_system_program(system_program_account: &AccountInfo) -> ProgramResult {
        if system_program_account.key != &system_program::ID {
            Err(FreeTunnelError::InvalidSystemProgram.into())
        } else {
            Ok(())
        }
    }

    fn assert_token_program(token_program_account: &AccountInfo) -> ProgramResult {
        if token_program_account.key != &spl_token::id() {
            Err(FreeTunnelError::InvalidTokenProgram.into())
        } else {
            Ok(())
        }
    }

    fn assert_token_mint_valid(token_mint: &AccountInfo) -> ProgramResult {
        if token_mint.owner != &spl_token::id() {
            Err(FreeTunnelError::InvalidTokenMint.into())
        } else {
            Ok(())
        }
    }

}
