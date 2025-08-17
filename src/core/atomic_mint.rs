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
    core::{permissions::Permissions, req_helpers::ReqId},
    error::FreeTunnelError,
    state::{ProposedBurn, ProposedMint},
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

    pub(crate) fn check_propose_mint<'a>(
        data_account_tokens_proposers: &AccountInfo<'a>,
        proposer: &Pubkey,
        req_id: &ReqId,
    ) -> ProgramResult {
        if req_id.action() & 0x0f != 1 {
            Err(FreeTunnelError::NotLockMint.into())
        } else {
            Permissions::assert_only_proposer(data_account_tokens_proposers, proposer)?;
            req_id.assert_to_chain_only()
        }
    }

    pub(crate) fn check_propose_mint_from_burn<'a>(
        data_account_tokens_proposers: &AccountInfo<'a>,
        proposer: &Pubkey,
        req_id: &ReqId,
    ) -> ProgramResult {
        if req_id.action() & 0x0f != 3 {
            Err(FreeTunnelError::NotBurnMint.into())
        } else {
            Permissions::assert_only_proposer(data_account_tokens_proposers, proposer)?;
            req_id.assert_to_chain_only()
        }
    }

    pub(crate) fn check_propose_burn<'a>(req_id: &ReqId) -> ProgramResult {
        if req_id.action() & 0x0f != 2 {
            Err(FreeTunnelError::NotBurnUnlock.into())
        } else {
            req_id.assert_to_chain_only()
        }
    }

    pub(crate) fn check_propose_burn_from_mint<'a>(req_id: &ReqId) -> ProgramResult {
        if req_id.action() & 0x0f != 3 {
            Err(FreeTunnelError::NotBurnMint.into())
        } else {
            req_id.assert_from_chain_only()
        }
    }

    pub(crate) fn propose_mint_internal<'a>(
        program_id: &Pubkey,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_mint: &AccountInfo<'a>,
        req_id: &ReqId,
        recipient: &Pubkey,
    ) -> ProgramResult {
        // Check conditions
        req_id.checked_created_time()?;
        if !data_account_proposed_mint.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }

        // Write proposed-lock data
        DataAccountUtils::create_related_account(
            program_id,
            payer_account,
            data_account_proposed_mint,
            Constants::PREFIX_MINT,
            &req_id.data,
            size_of::<ProposedMint>() + Constants::SIZE_LENGTH,
        )?;
        DataAccountUtils::write_account_data(
            data_account_proposed_mint,
            ProposedMint { inner: *recipient },
        )?;

        // Check amount & token index
        req_id.checked_amount(data_account_tokens_proposers)?;
        req_id.checked_token_index(data_account_tokens_proposers)?;
        Ok(())
    }

    pub(crate) fn execute_mint_internal<'a>(
        _program_id: &Pubkey,
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
        // Check conditions
        let recipient =
            DataAccountUtils::read_account_data::<ProposedMint>(data_account_proposed_mint)?.inner;
        if recipient == Constants::EXECUTED_PLACEHOLDER {
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

        // Update proposed-mint data
        DataAccountUtils::write_account_data(
            data_account_proposed_mint,
            ProposedMint {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Check token match
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_tokens_proposers)?;
        Self::check_token_account_match_index(token_account_recipient, &expected_token_pubkey)?;
        if expected_token_pubkey != *account_token_mint.key {
            return Err(FreeTunnelError::TokenMismatch.into());
        }

        // Mint to recipient
        let mut accounts = vec![
            account_token_mint.clone(),
            token_account_recipient.clone(),
            account_multisig_owner.clone(),
        ];
        for w in account_multisig_wallets {
            accounts.push(w.clone());
        }
        invoke_signed(
            &mint_to(
                system_account_token_program.key,
                account_token_mint.key,
                token_account_recipient.key,
                account_multisig_owner.key,
                &account_multisig_wallets
                    .iter()
                    .map(|w| w.key)
                    .collect::<Vec<&Pubkey>>(),
                amount,
            )?,
            &accounts,
            &[],
        )
    }

    pub(crate) fn cancel_mint_internal<'a>(
        _program_id: &Pubkey,
        data_account_proposed_mint: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
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

    pub(crate) fn propose_burn_internal<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        payer_account: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_proposer: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
        req_id.checked_created_time()?;
        if !data_account_proposed_burn.data_is_empty() {
            return Err(FreeTunnelError::InvalidReqId.into());
        }
        if account_proposer.key == &Constants::EXECUTED_PLACEHOLDER {
            return Err(FreeTunnelError::InvalidProposer.into());
        }

        // Write proposed-burn data
        DataAccountUtils::create_related_account(
            program_id,
            payer_account,
            data_account_proposed_burn,
            Constants::PREFIX_BURN,
            &req_id.data,
            size_of::<ProposedBurn>() + Constants::SIZE_LENGTH,
        )?;
        DataAccountUtils::write_account_data(
            data_account_proposed_burn,
            ProposedBurn {
                inner: *account_proposer.key,
            },
        )?;

        // Transfer assets to contract
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_tokens_proposers)?;
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

    pub(crate) fn execute_burn_internal<'a>(
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
        // Check conditions
        let proposer =
            DataAccountUtils::read_account_data::<ProposedBurn>(data_account_proposed_burn)?.inner;
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

        // Update proposed-burn data
        DataAccountUtils::write_account_data(
            data_account_proposed_burn,
            ProposedBurn {
                inner: Constants::EXECUTED_PLACEHOLDER,
            },
        )?;

        // Burn token from contract
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_tokens_proposers)?;
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

    pub(crate) fn cancel_burn_internal<'a>(
        program_id: &Pubkey,
        system_account_token_program: &AccountInfo<'a>,
        data_account_tokens_proposers: &AccountInfo<'a>,
        data_account_proposed_burn: &AccountInfo<'a>,
        token_account_proposer: &AccountInfo<'a>,
        token_account_contract: &AccountInfo<'a>,
        account_contract_signer: &AccountInfo<'a>,
        req_id: &ReqId,
    ) -> ProgramResult {
        // Check conditions
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
        let amount = req_id.checked_amount(data_account_tokens_proposers)?;
        let (_, expected_token_pubkey, _) =
            req_id.checked_token_index_pubkey_decimal(data_account_tokens_proposers)?;
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
