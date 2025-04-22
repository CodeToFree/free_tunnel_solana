use solana_program::program_error::ProgramError;

#[derive(Debug)]
pub enum DataAccountError {
    PdaAccountMismatch = 201,
    PdaAccountNotWritable,
    PdaAccountAlreadyCreated,
    PdaAccountNotOwned,
}

impl From<DataAccountError> for ProgramError {
    fn from(e: DataAccountError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[derive(Debug)]
pub enum FreeTunnelError {
    DuplicatedExecutors = 301,
    SignerCannotBeZeroAddress,
    InvalidSignature,
}

impl From<FreeTunnelError> for ProgramError {
    fn from(e: FreeTunnelError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
