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
    // Utils & Signature
    DuplicatedExecutors = 301,
    SignerCannotBeZeroAddress,
    InvalidSignature,
    ArrayLengthNotEqual,
    NotMeetThreshold,
    ExecutorsNotYetActive,
    ExecutorsOfNextIndexIsActive,
    NonExecutors,

    // Req Helpers
    CreatedTimeTooEarly,
    CreatedTimeTooLate,
    TokenIndexNonExistent,
    AmountCannotBeZero,
    NotFromCurrentChain,
    NotToCurrentChain,

    // Permissions
    ThresholdMustBeGreaterThanZero,
    ActiveSinceShouldAfter36h,
    ActiveSinceShouldWithin5d,
}

impl From<FreeTunnelError> for ProgramError {
    fn from(e: FreeTunnelError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
