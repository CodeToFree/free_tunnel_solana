use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};
use spl_token::instruction::{burn, mint_to, transfer};

use crate::{constants::Constants, error::FreeTunnelError};

fn assert_contract_signer<'a>(
    program_id: &Pubkey,
    contract_signer: &AccountInfo<'a>,
) -> Result<u8, ProgramError> {
    let (expected_contract_pubkey, bump_seed) =
        Pubkey::find_program_address(&[Constants::CONTRACT_SIGNER], program_id);
    if expected_contract_pubkey != *contract_signer.key {
        return Err(FreeTunnelError::ContractSignerMismatch.into());
    }
    Ok(bump_seed)
}

pub(crate) fn transfer_to_contract<'a>(
    token_program: &AccountInfo<'a>,
    contract: &AccountInfo<'a>,
    from: &AccountInfo<'a>,
    from_signer: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    invoke_signed(
        &transfer(
            token_program.key,
            from.key,
            contract.key,
            from_signer.key,
            &[],
            amount,
        )?,
        &[
            from.clone(),
            contract.clone(),
            from_signer.clone(),
        ],
        &[],
    )?;
    Ok(())
}

pub(crate) fn transfer_from_contract<'a>(
    program_id: &Pubkey,
    token_program: &AccountInfo<'a>,
    contract_signer: &AccountInfo<'a>,
    contract: &AccountInfo<'a>,
    recipient: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let bump_seed = assert_contract_signer(program_id, contract_signer)?;
    invoke_signed(
        &transfer(
            token_program.key,
            contract.key,
            recipient.key,
            contract_signer.key,
            &[],
            amount,
        )?,
        &[
            contract.clone(),
            recipient.clone(),
            contract_signer.clone(),
        ],
        &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]],
    )?;
    Ok(())
}

pub(crate) fn mint_token<'a>(
    program_id: &Pubkey,
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    contract_signer: &AccountInfo<'a>,
    recipient: &AccountInfo<'a>,
    multisig_owner: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let bump_seed = assert_contract_signer(program_id, contract_signer)?;
    invoke_signed(
        &mint_to(
            token_program.key,
            token_mint.key,
            recipient.key,
            multisig_owner.key,
            &[contract_signer.key],
            amount,
        )?,
        &[
            token_mint.clone(),
            recipient.clone(),
            multisig_owner.clone(),
            contract_signer.clone(),
        ],
        &[&[Constants::CONTRACT_SIGNER, &[bump_seed]][..]],
    )?;
    Ok(())
}

pub(crate) fn burn_token<'a>(
    program_id: &Pubkey,
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    contract_signer: &AccountInfo<'a>,
    contract: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let bump_seed = assert_contract_signer(program_id, contract_signer)?;
    invoke_signed(
        &burn(
            token_program.key,
            contract.key,
            token_mint.key,
            contract_signer.key,
            &[],
            amount,
        )?,
        &[
            contract.clone(),
            token_mint.clone(),
            contract_signer.clone(),
        ],
        &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]],
    )?;
    Ok(())
}
