use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

use crate::processor::Processor;
entrypoint!(process_instruction);

pub mod constants;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

pub mod logic {
    pub mod atomic_lock;
    pub mod atomic_mint;
    pub mod permissions;
    pub mod req_helpers;
    pub mod token_ops;
}

#[cfg(test)]
pub mod test {
    pub mod req_helpers_test;
    pub mod utils_test;
}


pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process_instruction(program_id, accounts, instruction_data)
}
