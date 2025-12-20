use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program::invoke_signed, pubkey::Pubkey, sysvar::Sysvar,
    program_error::ProgramError,
};
use spl_token::instruction::{burn, mint_to, transfer};
use std::mem::size_of;

use crate::{
    constants::{Constants, EthAddress},
    error::FreeTunnelError,
    logic::{permissions::Permissions, req_helpers::ReqId},
    state::{BasicStorage, ProposedBurn, ProposedMint},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct AtomicMint;

impl AtomicMint {
    fn assert_contract_mode_is_mint<'a>(
        data_account_basic_storage: &AccountInfo<'a>,
    ) -> ProgramResult {
        let basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
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
        // Check conditions
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        req_id.assert_mint_side()?;
        let specific_action = req_id.action() & 0x0f;
        if specific_action != 1 && specific_action != 3 {
            return Err(FreeTunnelError::NotLockMint.into());
        }
        Permissions::assert_only_proposer(data_account_basic_storage, account_proposer, true)?;
        req_id.checked_created_time()?;
        if !data_account_proposed_mint.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        if *recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidRecipient.into());
        }

        // Check amount & token index
        let (_, decimal) = req_id.get_checked_token(data_account_basic_storage, None)?;
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

        msg!(
            "TokenMintProposed: req_id={}, recipient={}",
            hex::encode(req_id.data),
            recipient
        );
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
        // Check conditions
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let recipient =
            DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Check signatures
        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::assert_multisig_valid(
            data_account_executors,
            &message,
            signatures,
            executors,
        )?;

        // Update proposed-mint data
        DataAccountUtils::write_account_data(
            data_account_proposed_mint,
            ProposedMint {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Check token match
        let (_, decimal) =
            req_id.get_checked_token(data_account_basic_storage, Some(token_account_recipient))?;
        let amount = req_id.get_checked_amount(decimal)?;

        // Mint to recipient
        let (expected_contract_pubkey, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if expected_contract_pubkey != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &mint_to(
                token_program.key,
                token_mint.key,
                token_account_recipient.key,
                account_multisig_owner.key, // The 1/3 multisig account is the authority
                &[account_contract_signer.key], // The PDA is the ONLY signer required for this CPI
                amount,
            )?,
            // The accounts required by the `mint_to` CPI
            &[
                token_mint.clone(),
                token_account_recipient.clone(),
                account_multisig_owner.clone(),
                account_contract_signer.clone(), // The PDA account must be passed to the CPI
            ],
            &[&[Constants::CONTRACT_SIGNER, &[bump_seed]][..]],
        )?;

        msg!(
            "TokenMintExecuted: req_id={}, recipient={}",
            hex::encode(req_id.data),
            recipient
        );
        Ok(())
    }

    pub(crate) fn cancel_mint<'a>(
        program_id: &Pubkey,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        account_refund: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let recipient =
            DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        let now = Clock::get()?.unix_timestamp;
        if now <= (req_id.created_time() + Constants::EXPIRE_EXTRA_PERIOD) as i64 {
            return Err(FreeTunnelError::WaitUntilExpired.into());
        }

        Permissions::assert_only_proposer(data_account_basic_storage, account_refund, false)?;
        DataAccountUtils::close_account(program_id, data_account_proposed_mint, account_refund)?;

        msg!(
            "TokenMintCancelled: req_id={}, recipient={}",
            hex::encode(req_id.data),
            recipient
        );
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
        // Check conditions
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let specific_action = req_id.action() & 0x0f;
        match specific_action {
            2 => {
                req_id.assert_mint_side()?;
            }
            3 => {
                req_id.assert_mint_opposite_side()?;
            }
            _ => return Err(FreeTunnelError::NotBurnUnlock.into()),
        }
        // Check signers
        if !account_proposer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        req_id.checked_created_time()?;
        if !data_account_proposed_burn.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        if account_proposer.key == &Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidProposer.into());
        }

        // Check amount & token
        let (_, decimal) =
            req_id.get_checked_token(data_account_basic_storage, Some(token_account_proposer))?;
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
        invoke_signed(
            &transfer(
                token_program.key,
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
        )?;

        msg!(
            "TokenBurnProposed: req_id={}, proposer={}",
            hex::encode(req_id.data),
            account_proposer.key
        );
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
        // Check conditions
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let proposer =
            DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Check signatures
        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::assert_multisig_valid(
            data_account_executors,
            &message,
            signatures,
            executors,
        )?;

        // Update proposed-burn data
        DataAccountUtils::write_account_data(
            data_account_proposed_burn,
            ProposedBurn {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Burn token from contract
        let (_, decimal) =
            req_id.get_checked_token(data_account_basic_storage, Some(token_account_contract))?;
        let amount = req_id.get_checked_amount(decimal)?;
        let (expected_contract_pubkey, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if expected_contract_pubkey != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &burn(
                token_program.key,
                token_account_contract.key,
                token_mint.key,
                account_contract_signer.key,
                &[],
                amount,
            )?,
            &[
                token_account_contract.clone(),
                token_mint.clone(),
                account_contract_signer.clone(),
            ],
            &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]],
        )?;

        msg!(
            "TokenBurnExecuted: req_id={}, proposer={}",
            hex::encode(req_id.data),
            proposer
        );
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
        // Check conditions
        Self::assert_contract_mode_is_mint(data_account_basic_storage)?;
        let proposer =
            DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        let now = Clock::get()?.unix_timestamp;
        if now <= (req_id.created_time() + Constants::EXPIRE_PERIOD) as i64 {
            return Err(FreeTunnelError::WaitUntilExpired.into());
        }

        // Check amount & token
        let (_, decimal) =
            req_id.get_checked_token(data_account_basic_storage, Some(token_account_contract))?;
        let amount = req_id.get_checked_amount(decimal)?;

        Permissions::assert_only_proposer(data_account_basic_storage, account_refund, false)?;
        DataAccountUtils::close_account(program_id, data_account_proposed_burn, account_refund)?;

        // Refund token
        let (expected_contract_pubkey, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if expected_contract_pubkey != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &transfer(
                token_program.key,
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
        )?;

        msg!(
            "TokenBurnCancelled: req_id={}, proposer={}",
            hex::encode(req_id.data),
            proposer
        );
        Ok(())
    }
}
