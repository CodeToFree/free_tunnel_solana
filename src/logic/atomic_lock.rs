use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};
use std::mem::size_of;

use crate::{
    constants::{Constants, EthAddress},
    error::FreeTunnelError,
    logic::{permissions::Permissions, req_helpers::ReqId, token_ops},
    state::{BasicStorage, ProposedLock, ProposedUnlock},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct AtomicLock;

impl AtomicLock {
    fn assert_contract_mode_is_lock<'a>(
        data_account_basic_storage: &AccountInfo<'a>,
    ) -> ProgramResult {
        let basic_storage: BasicStorage = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        match basic_storage.mint_or_lock {
            true => Err(FreeTunnelError::NotLockContract.into()),
            false => Ok(()),
        }
    }

    pub(crate) fn propose_lock<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        token_program: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>, // signer
        token_account_contract: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_lock(data_account_basic_storage)?;
        req_id.assert_mint_opposite_side()?;
        if req_id.action() & 0x0f != 1 { return Err(FreeTunnelError::NotLockMint.into()); }

        if !account_proposer.is_signer { return Err(ProgramError::MissingRequiredSignature); }
        req_id.checked_created_time()?;
        if !data_account_proposed_lock.data_is_empty() { return Err(FreeTunnelError::InvalidReqId.into()); }
        if account_proposer.key == &Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidProposer.into());
        }

        // Check amount & token
        let (_, decimal) = req_id.get_checked_token(data_account_basic_storage, Some(token_account_proposer))?;
        let amount = req_id.get_checked_amount(decimal)?;

        // Write proposed-lock data
        DataAccountUtils::create_data_account(
            program_id,
            system_program,
            account_proposer,
            data_account_proposed_lock,
            Constants::PREFIX_LOCK,
            &req_id.data,
            size_of::<ProposedLock>() + Constants::SIZE_LENGTH,
            ProposedLock { inner: *account_proposer.key },
        )?;

        // Deposit token
        token_ops::transfer_to_contract(token_program, token_account_proposer, token_account_contract, account_proposer, amount)?;

        msg!("TokenLockProposed: req_id={}, proposer={}", hex::encode(req_id.data), account_proposer.key);
        Ok(())
    }

    pub(crate) fn execute_lock<'a>(
        _program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        data_account_executors: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_lock(data_account_basic_storage)?;
        let proposer = DataAccountUtils::read_account_data::<ProposedLock>(data_account_proposed_lock)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::assert_multisig_valid(data_account_executors, &message, signatures, executors)?;

        // Update proposed-lock data
        DataAccountUtils::write_account_data(
            data_account_proposed_lock,
            ProposedLock { inner: Constants::EXECUTED_PLACEHOLDER },
        )?;

        // Update locked-balance data
        let (token_index, decimal) = req_id.get_checked_token(data_account_basic_storage, None)?;
        let amount = req_id.get_checked_amount(decimal)?;
        Self::update_locked_balance(data_account_basic_storage, token_index, amount, true)?;

        msg!("TokenLockExecuted: req_id={}, proposer={}", hex::encode(req_id.data), proposer);
        Ok(())
    }

    pub(crate) fn cancel_lock<'a>(
        program_id: &Pubkey,
        token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_lock: &AccountInfo<'a>,
        account_refund: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_lock(data_account_basic_storage)?;
        let proposer = DataAccountUtils::read_account_data::<ProposedLock>(data_account_proposed_lock)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        let now = Clock::get()?.unix_timestamp;
        if now <= (req_id.created_time() + Constants::EXPIRE_PERIOD) as i64 { return Err(FreeTunnelError::WaitUntilExpired.into()); }

        let (_, decimal) = req_id.get_checked_token(data_account_basic_storage, Some(token_account_contract))?;
        let amount = req_id.get_checked_amount(decimal)?;

        Permissions::assert_only_proposer(data_account_basic_storage, account_refund, false)?;
        DataAccountUtils::close_account(program_id, data_account_proposed_lock, account_refund)?;

        // Refund token
        token_ops::transfer_from_contract(
            program_id,
            token_program,
            account_contract_signer,
            token_account_contract,
            token_account_proposer,
            amount,
        )?;

        msg!("TokenLockCancelled: req_id={}, proposer={}", hex::encode(req_id.data), proposer);
        Ok(())
    }

    pub(crate) fn propose_unlock<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>, // signer
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_lock(data_account_basic_storage)?;
        req_id.assert_mint_opposite_side()?;
        if req_id.action() & 0x0f != 2 { return Err(FreeTunnelError::NotBurnUnlock.into()); }

        Permissions::assert_only_proposer(data_account_basic_storage, account_proposer, true)?;
        req_id.checked_created_time()?;
        if !data_account_proposed_unlock.data_is_empty() { return Err(FreeTunnelError::InvalidReqId.into()); }
        if *recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidRecipient.into());
        }

        // Check amount & token
        let (token_index, decimal) = req_id.get_checked_token(data_account_basic_storage, None)?;
        let amount = req_id.get_checked_amount(decimal)?;
        Self::update_locked_balance(data_account_basic_storage, token_index, amount, false)?;

        // Write proposed-unlock data
        DataAccountUtils::create_data_account(
            program_id,
            system_program,
            account_proposer,
            data_account_proposed_unlock,
            Constants::PREFIX_UNLOCK,
            &req_id.data,
            size_of::<ProposedUnlock>() + Constants::SIZE_LENGTH,
            ProposedUnlock { inner: *recipient },
        )?;

        msg!("TokenUnlockProposed: req_id={}, recipient={}", hex::encode(req_id.data), recipient);
        Ok(())
    }

    pub(crate) fn execute_unlock<'a>(
        program_id: &Pubkey,
        token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        token_account_recipient: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        data_account_executors: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_lock(data_account_basic_storage)?;
        let recipient = DataAccountUtils::read_account_data::<ProposedUnlock>(data_account_proposed_unlock)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER { return Err(FreeTunnelError::InvalidReqId.into()); }

        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::assert_multisig_valid(data_account_executors, &message, signatures, executors)?;

        // Update proposed-unlock data
        DataAccountUtils::write_account_data(
            data_account_proposed_unlock,
            ProposedUnlock { inner: Constants::EXECUTED_PLACEHOLDER },
        )?;

        // Unlock token to recipient
        let (_, decimal) = req_id.get_checked_token(data_account_basic_storage, Some(token_account_contract))?;
        let amount = req_id.get_checked_amount(decimal)?;

        token_ops::transfer_from_contract(
            program_id,
            token_program,
            account_contract_signer,
            token_account_contract,
            token_account_recipient,
            amount,
        )?;

        msg!("TokenUnlockExecuted: req_id={}, recipient={}", hex::encode(req_id.data), recipient);
        Ok(())
    }

    pub(crate) fn cancel_unlock<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_unlock: &AccountInfo<'a>,
        account_refund: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_lock(data_account_basic_storage)?;
        let recipient = DataAccountUtils::read_account_data::<ProposedUnlock>(data_account_proposed_unlock)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        let now = Clock::get()?.unix_timestamp;
        if now <= (req_id.created_time() + Constants::EXPIRE_EXTRA_PERIOD) as i64 { return Err(FreeTunnelError::WaitUntilExpired.into()); }

        // Update locked-balance data
        let (token_index, decimal) = req_id.get_checked_token(data_account_basic_storage, None)?;
        let amount = req_id.get_checked_amount(decimal)?;
        Self::update_locked_balance(data_account_basic_storage, token_index, amount, true)?;

        Permissions::assert_only_proposer(data_account_basic_storage, account_refund, false)?;
        DataAccountUtils::close_account(program_id, data_account_proposed_unlock, account_refund)?;

        msg!("TokenUnlockCancelled: req_id={}, recipient={}", hex::encode(req_id.data), recipient);
        Ok(())
    }


    fn update_locked_balance(
        data_account_basic_storage: &AccountInfo,
        token_index: u8,
        amount: u64,
        is_add: bool,
    ) -> ProgramResult {
        let mut basic_storage: BasicStorage = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        let locked_balance = basic_storage.locked_balance.get_mut(token_index).ok_or(FreeTunnelError::TokenIndexNonExistent)?;
        if is_add {
            *locked_balance = locked_balance.checked_add(amount).ok_or(FreeTunnelError::ArithmeticOverflow)?;
        } else {
            *locked_balance = locked_balance.checked_sub(amount).ok_or(FreeTunnelError::LockedBalanceInsufficient)?;
        }
        DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)
    }
}
