use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, program::invoke_signed,
    pubkey::Pubkey, sysvar::Sysvar,
};
use spl_token::{
    instruction::{burn, mint_to, transfer},
    state::{Account as TokenAccount, GenericTokenAccount},
};
use std::mem::size_of;

use crate::{
    constants::{Constants, EthAddress},
    logic::{permissions::Permissions, req_helpers::ReqId},
    error::FreeTunnelError,
    state::{BasicStorage, ProposedBurn, ProposedMint},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct AtomicMint;

impl AtomicMint {
    fn check_token_account_match_index<'a>(
        token_account: &AccountInfo<'a>,
        expected_token_pubkey: &Pubkey,
    ) -> ProgramResult {
        let token_account_data = token_account.data.borrow();
        match TokenAccount::valid_account_data(&token_account_data) {
            true => {
                let token_pubkey = TokenAccount::unpack_account_mint_unchecked(&token_account_data);
                if expected_token_pubkey != token_pubkey {
                    Err(FreeTunnelError::TokenMismatch.into())
                } else {
                    Ok(())
                }
            }
            false => Err(FreeTunnelError::InvalidTokenAccount.into()),
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

    pub(crate) fn propose_mint<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }

        // Check conditions
        let specific_action = req_id.action() & 0x0f;
        if specific_action != 1 && specific_action != 3 {
            return Err(FreeTunnelError::InvalidAction.into());
        } else {
            Permissions::assert_only_proposer(data_account_basic_storage, account_proposer)?;
            req_id.checked_created_time()?;
            req_id.assert_to_chain_only()?;
        }

        Self::check_is_mint_contract(data_account_basic_storage)?;
        if !data_account_proposed_mint.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

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

        // Check amount & token index
        req_id.checked_amount(data_account_basic_storage)?;
        req_id.checked_token_index_pubkey_decimal(data_account_basic_storage)?;
        Ok(())
    }

    pub(crate) fn execute_mint<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_recipient: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_proposed_mint: &AccountInfo<'a>,
        data_account_executors: &AccountInfo,
        account_token_mint: &AccountInfo<'a>,
        account_multisig_owner: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
    ) -> ProgramResult {
        // Check conditions
        Self::check_is_mint_contract(data_account_basic_storage)?;
        let recipient =
            DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Check signatures
        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::check_multi_signatures(
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
        let amount = req_id.checked_amount(data_account_basic_storage)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_basic_storage)?;
        Self::check_token_account_match_index(token_account_recipient, &expected_token_pubkey)?;
        if expected_token_pubkey != *account_token_mint.key {
            return Err(FreeTunnelError::TokenMismatch.into());
        }

        // Mint to recipient
        let (pda_key, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if pda_key != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &mint_to(
                system_account_token_program.key,
                account_token_mint.key,
                token_account_recipient.key,
                account_multisig_owner.key, // The 1/3 multisig account is the authority
                &[account_contract_signer.key], // The PDA is the ONLY signer required for this CPI
                amount,
            )?,
            // The accounts required by the `mint_to` CPI
            &[
                account_token_mint.clone(),
                token_account_recipient.clone(),
                account_multisig_owner.clone(),
                account_contract_signer.clone(), // The PDA account must be passed to the CPI
            ],
            &[&[Constants::CONTRACT_SIGNER, &[bump_seed]][..]],
        )
    }

    pub(crate) fn cancel_mint<'a>(
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        Self::check_is_mint_contract(data_account_basic_storage)?;
        let recipient =
            DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        let now = Clock::get()?.unix_timestamp;
        if now < (req_id.created_time() + Constants::EXPIRE_EXTRA_PERIOD) as i64 {
            return Err(FreeTunnelError::WaitUntilExpired.into());
        }

        // Update proposed-mint data
        DataAccountUtils::write_account_data(
            data_account_proposed_mint,
            ProposedMint {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )
    }

    pub(crate) fn propose_burn<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        system_account_token_program: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check signers
        if !account_proposer.is_signer {
            return Err(FreeTunnelError::ProposerNotSigner.into());
        }
        if account_proposer.key == &Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidProposer.into());
        }

        // Check conditions
        let specific_action = req_id.action() & 0x0f;
        match specific_action {
            2 | 3 => {
                req_id.checked_created_time()?;
                let (check, err) = if specific_action == 2 {
                    (req_id.assert_to_chain_only(), FreeTunnelError::EHubNotMintSide)
                } else {
                    (req_id.assert_from_chain_only(), FreeTunnelError::EHubNotMintOppositeSide)
                };
                check.map_err(|_| err)?;
            }
            _ => return Err(FreeTunnelError::InvalidAction.into())
        }

        Self::check_is_mint_contract(data_account_basic_storage)?;
        if !data_account_proposed_burn.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

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
        let amount = req_id.checked_amount(data_account_basic_storage)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_basic_storage)?;
        Self::check_token_account_match_index(token_account_proposer, &expected_token_pubkey)?;
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

    pub(crate) fn execute_burn<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_proposed_burn: &AccountInfo<'a>,
        data_account_executors: &AccountInfo,
        account_token_mint: &AccountInfo<'a>,
        req_id: &ReqId,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
    ) -> ProgramResult {
        // Check conditions
        Self::check_is_mint_contract(data_account_basic_storage)?;
        let proposer =
            DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Check signatures
        let message = req_id.msg_from_req_signing_message();
        SignatureUtils::check_multi_signatures(
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
        let amount = req_id.checked_amount(data_account_basic_storage)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_basic_storage)?;
        Self::check_token_account_match_index(token_account_contract, &expected_token_pubkey)?;
        let (expected_contract_pubkey, bump_seed) =
            Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
        if expected_contract_pubkey != *account_contract_signer.key {
            return Err(FreeTunnelError::ContractSignerMismatch.into());
        }
        invoke_signed(
            &burn(
                system_account_token_program.key,
                token_account_contract.key,
                account_token_mint.key,
                account_contract_signer.key,
                &[],
                amount,
            )?,
            &[
                token_account_contract.clone(),
                account_token_mint.clone(),
                account_contract_signer.clone(),
            ],
            &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]],
        )
    }

    pub(crate) fn cancel_burn<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        Self::check_is_mint_contract(data_account_basic_storage)?;
        let proposer =
            DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
        if proposer == Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        let now = Clock::get()?.unix_timestamp;
        if now < (req_id.created_time() + Constants::EXPIRE_PERIOD) as i64 {
            return Err(FreeTunnelError::WaitUntilExpired.into());
        }

        // Update proposed-burn data
        DataAccountUtils::write_account_data(
            data_account_proposed_burn,
            ProposedBurn {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Update locked-balance data
        let amount = req_id.checked_amount(data_account_basic_storage)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_basic_storage)?;
        Self::check_token_account_match_index(token_account_contract, &expected_token_pubkey)?;
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
}
