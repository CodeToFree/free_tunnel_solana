use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke,
    program::invoke_signed, program_error::ProgramError, pubkey::Pubkey,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use spl_token::instruction as spl_instruction;
use spl_token_2022::instruction as spl_2022_instruction;

use crate::{
    constants::Constants,
    error::FreeTunnelError,
    state::BasicStorage,
    utils::DataAccountUtils,
};

pub(crate) enum TokenProgramKind {
    Token,
    Token2022,
}

fn token_program_kind(token_program: &AccountInfo) -> Result<TokenProgramKind, ProgramError> {
    if token_program.key == &spl_token::id() {
        Ok(TokenProgramKind::Token)
    } else if token_program.key == &spl_token_2022::id() {
        Ok(TokenProgramKind::Token2022)
    } else {
        Err(FreeTunnelError::InvalidTokenProgram.into())
    }
}

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

pub(crate) fn assert_is_ata(
    token_program: &AccountInfo,
    token_account: &AccountInfo,
    owner_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
) -> ProgramResult {
    let expected = get_associated_token_address_with_program_id(
        owner_pubkey, 
        mint_pubkey, 
        token_program.key
    );
    if token_account.key != &expected {
        return Err(FreeTunnelError::InvalidTokenAccount.into());
    }
    Ok(())
}

pub(crate) fn assert_is_contract_ata<'a>(
    data_account_basic_storage: &AccountInfo<'a>,
    token_index: u8,
    token_account_contract: &AccountInfo<'a>,
) -> ProgramResult {
    let basic_storage: BasicStorage = DataAccountUtils::read_account_data(data_account_basic_storage)?;
    let expected = basic_storage.vaults.get(token_index).ok_or(FreeTunnelError::TokenIndexNonExistent)?;
    if token_account_contract.key != expected {
        return Err(FreeTunnelError::InvalidTokenAccount.into());
    }
    Ok(())
}

pub(crate) fn create_token_account_contract<'a>(
    system_program: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    token_account_contract: &AccountInfo<'a>,
    account_contract_signer: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    rent_sysvar: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    assert_is_ata(token_program, token_account_contract, account_contract_signer.key, token_mint.key)?;

    let ix = create_associated_token_account_idempotent(
        payer.key,
        account_contract_signer.key,
        token_mint.key,
        token_program.key,
    );

    invoke(
        &ix,
        &[
            system_program.clone(),
            token_program.clone(),
            payer.clone(),
            token_account_contract.clone(),
            account_contract_signer.clone(),
            token_mint.clone(),
            rent_sysvar.clone(),
        ],
    )?;

    Ok(())
}

pub(crate) fn transfer_to_contract<'a>(
    token_program: &AccountInfo<'a>,
    contract: &AccountInfo<'a>,
    from: &AccountInfo<'a>,
    from_signer: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let ix = match token_program_kind(token_program)? {
        TokenProgramKind::Token => spl_instruction::transfer(
            token_program.key,
            from.key,
            contract.key,
            from_signer.key,
            &[],
            amount,
        )?,
        TokenProgramKind::Token2022 => spl_2022_instruction::transfer(
            token_program.key,
            from.key,
            contract.key,
            from_signer.key,
            &[],
            amount,
        )?,
    };
    invoke_signed(&ix, &[from.clone(), contract.clone(), from_signer.clone()], &[])?;
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
    let ix = match token_program_kind(token_program)? {
        TokenProgramKind::Token => spl_instruction::transfer(
            token_program.key,
            contract.key,
            recipient.key,
            contract_signer.key,
            &[],
            amount,
        )?,
        TokenProgramKind::Token2022 => spl_2022_instruction::transfer(
            token_program.key,
            contract.key,
            recipient.key,
            contract_signer.key,
            &[],
            amount,
        )?,
    };
    invoke_signed(&ix, &[contract.clone(), recipient.clone(), contract_signer.clone()], &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]])?;
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
    let ix = match token_program_kind(token_program)? {
        TokenProgramKind::Token => spl_instruction::mint_to(
            token_program.key,
            token_mint.key,
            recipient.key,
            multisig_owner.key,
            &[contract_signer.key],
            amount,
        )?,
        TokenProgramKind::Token2022 => spl_2022_instruction::mint_to(
            token_program.key,
            token_mint.key,
            recipient.key,
            multisig_owner.key,
            &[contract_signer.key],
            amount,
        )?,
    };
    invoke_signed(
        &ix,
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
    let ix = match token_program_kind(token_program)? {
        TokenProgramKind::Token => spl_instruction::burn(
            token_program.key,
            contract.key,
            token_mint.key,
            contract_signer.key,
            &[],
            amount,
        )?,
        TokenProgramKind::Token2022 => spl_2022_instruction::burn(
            token_program.key,
            contract.key,
            token_mint.key,
            contract_signer.key,
            &[],
            amount,
        )?,
    };
    invoke_signed(&ix, &[contract.clone(), token_mint.clone(), contract_signer.clone()], &[&[Constants::CONTRACT_SIGNER, &[bump_seed]]])?;
    Ok(())
}
