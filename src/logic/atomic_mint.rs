use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};
use std::mem::size_of;

use crate::{
    constants::{Constants, EthAddress},
    error::FreeTunnelError,
    logic::{permissions::Permissions, req_helpers::ReqId, token_ops},
    state::{BasicStorage, ProposedBurn, ProposedMint},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct AtomicMint;

impl AtomicMint {
    fn assert_contract_mode_is_mint<'a>(
        data_account_basic_storage: &AccountInfo<'a>,
    ) -> ProgramResult {
        let basic_storage: BasicStorage = DataAccountUtils::read_account_data(data_account_basic_storage)?;
        match basic_storage.mint_or_lock {
            true => Ok(()),
            false => Err(FreeTunnelError::NotMintContract.into()),
        }
    }

    pub(crate) fn propose_mint<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        req_id.assert_mint_side()?;
        let specific_action = req_id.action() & 0x0f;
        if specific_action != 1 && specific_action != 3 { return Err(FreeTunnelError::NotLockMint.into()); }

        Permissions::assert_only_proposer(data_account_basic_storage, account_proposer, true)?;
        req_id.checked_created_time()?;
        if !data_account_proposed_mint.data_is_empty() { return Err(FreeTunnelError::ReqIdOccupied.into()); }
        if *recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidRecipient.into());
        }

        // Check amount & token index
        let (_, decimal, _) = req_id.get_checked_token(data_account_basic_storage, None)?;
        req_id.get_checked_amount(decimal)?;

        // Write proposed-lock data
        DataAccountUtils::create_data_account(
            program_id,
            system_program,
            account_proposer,
            data_account_proposed_mint,
            Constants::PREFIX_MINT,
            &req_id.data,
            size_of::<ProposedMint>() + Constants::SIZE_LENGTH,
            ProposedMint { inner: *recipient },
        )?;

        msg!("TokenMintProposed: req_id={}, recipient={}", hex::encode(req_id.data), recipient);
        Ok(())
    }

    pub(crate) fn execute_mint<'a>(
        program_id: &Pubkey,
        token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_recipient: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        data_account_executors: &AccountInfo<'a>,
        token_mint: &AccountInfo<'a>,
        account_multisig_owner: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let recipient = DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::ReqIdExecuted.into());
        }

        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::assert_multisig_valid(data_account_executors, &message, signatures, executors)?;

        // Update proposed-mint data
        DataAccountUtils::write_account_data(
            data_account_proposed_mint,
            ProposedMint { inner: Constants::EXECUTED_PLACEHOLDER },
        )?;

        // Check token match
        let (_, decimal, mint_pubkey) = req_id.get_checked_token(data_account_basic_storage, None)?;
        let amount = req_id.get_checked_amount(decimal)?;
        if token_mint.key != &mint_pubkey {
            return Err(FreeTunnelError::TokenMismatch.into());
        }

        // Mint to recipient
        token_ops::assert_is_ata(token_program, token_account_recipient, &recipient, &mint_pubkey)?;
        token_ops::mint_token(
            program_id,
            token_program,
            token_mint,
            token_account_recipient,
            account_multisig_owner,
            account_contract_signer,
            amount,
        )?;

        msg!("TokenMintExecuted: req_id={}, recipient={}", hex::encode(req_id.data), recipient);
        Ok(())
    }

    pub(crate) fn cancel_mint<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        account_refund: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let recipient = DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::ReqIdExecuted.into());
        }

        let now = Clock::get()?.unix_timestamp;
        if now <= (req_id.created_time() + Constants::EXPIRE_EXTRA_PERIOD) as i64 { return Err(FreeTunnelError::WaitUntilExpired.into()); }

        Permissions::assert_only_proposer(data_account_basic_storage, account_refund, false)?;
        DataAccountUtils::close_account(program_id, data_account_proposed_mint, account_refund)?;

        msg!("TokenMintCancelled: req_id={}, recipient={}", hex::encode(req_id.data), recipient);
        Ok(())
    }

    pub(crate) fn propose_burn<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        token_program: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let specific_action = req_id.action() & 0x0f;
        match specific_action {
            2 => { req_id.assert_mint_side()?; }
            3 => { req_id.assert_mint_opposite_side()?; }
            _ => return Err(FreeTunnelError::NotBurnUnlock.into()),
        }

        if !account_proposer.is_signer { return Err(ProgramError::MissingRequiredSignature); }
        req_id.checked_created_time()?;
        if !data_account_proposed_burn.data_is_empty() { return Err(FreeTunnelError::ReqIdOccupied.into()); }
        if account_proposer.key == &Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidProposer.into());
        }

        // Check amount & token
        let (token_index, decimal, _) = req_id.get_checked_token(data_account_basic_storage, Some(token_account_proposer))?;
        let amount = req_id.get_checked_amount(decimal)?;

        // Write proposed-burn data
        DataAccountUtils::create_data_account(
            program_id,
            system_program,
            account_proposer,
            data_account_proposed_burn,
            Constants::PREFIX_BURN,
            &req_id.data,
            size_of::<ProposedBurn>() + Constants::SIZE_LENGTH,
            ProposedBurn { inner: *account_proposer.key },
        )?;

        // Transfer assets to contract
        token_ops::assert_is_contract_ata(data_account_basic_storage, token_index, token_account_contract)?;
        token_ops::transfer_to_contract(token_program, token_account_proposer, token_account_contract, account_proposer, amount)?;

        msg!("TokenBurnProposed: req_id={}, proposer={}", hex::encode(req_id.data), account_proposer.key);
        Ok(())
    }

    pub(crate) fn execute_burn<'a>(
        program_id: &Pubkey,
        token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        data_account_executors: &AccountInfo<'a>,
        token_mint: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let proposer = DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::ReqIdExecuted.into());
        }

        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::assert_multisig_valid(data_account_executors, &message, signatures, executors)?;

        // Update proposed-burn data
        DataAccountUtils::write_account_data(
            data_account_proposed_burn,
            ProposedBurn { inner: Constants::EXECUTED_PLACEHOLDER },
        )?;

        // Burn token from contract
        let (token_index, decimal, mint_pubkey) = req_id.get_checked_token(data_account_basic_storage, None)?;
        let amount = req_id.get_checked_amount(decimal)?;
        if token_mint.key != &mint_pubkey {
            return Err(FreeTunnelError::TokenMismatch.into());
        }

        token_ops::assert_is_contract_ata(data_account_basic_storage, token_index, token_account_contract)?;
        token_ops::burn_token(
            program_id,
            token_program,
            token_mint,
            account_contract_signer,
            token_account_contract,
            amount,
        )?;

        msg!("TokenBurnExecuted: req_id={}, proposer={}", hex::encode(req_id.data), proposer);
        Ok(())
    }

    pub(crate) fn cancel_burn<'a>(
        program_id: &Pubkey,
        token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        account_refund: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let proposer = DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::ReqIdExecuted.into());
        }

        let now = Clock::get()?.unix_timestamp;
        if now <= (req_id.created_time() + Constants::EXPIRE_PERIOD) as i64 { return Err(FreeTunnelError::WaitUntilExpired.into()); }

        // Check amount & token
        let (token_index, decimal, mint_pubkey) = req_id.get_checked_token(data_account_basic_storage, None)?;
        let amount = req_id.get_checked_amount(decimal)?;

        Permissions::assert_only_proposer(data_account_basic_storage, account_refund, false)?;
        DataAccountUtils::close_account(program_id, data_account_proposed_burn, account_refund)?;

        // Refund token
        token_ops::assert_is_contract_ata(data_account_basic_storage, token_index, token_account_contract)?;
        token_ops::assert_is_ata(token_program, token_account_proposer, &proposer, &mint_pubkey)?;
        token_ops::transfer_from_contract(
            program_id,
            token_program,
            account_contract_signer,
            token_account_contract,
            token_account_proposer,
            amount,
        )?;

        msg!("TokenBurnCancelled: req_id={}, proposer={}", hex::encode(req_id.data), proposer);
        Ok(())
    }
}
