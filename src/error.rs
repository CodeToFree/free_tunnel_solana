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
    CreatedTimeTooEarly = 401,
    CreatedTimeTooLate,
    TokenIndexOccupied,
    TokenIndexCannotBeZero,
    TokenIndexNonExistent,
    AmountCannotBeZero,
    NotFromCurrentChain,
    NotToCurrentChain,

    // Permissions
    NotAdmin = 501,
    NotProposer,
    AlreadyProposer,
    NotExistingProposer,
    ExecutorsAlreadyInitialized,
    ThresholdMustBeGreaterThanZero,
    ActiveSinceShouldAfter36h,
    ActiveSinceShouldWithin5d,
    FailedToOverwriteExistingExecutors,

    // Atomic Lock & Mint
    NotLockMint = 601,
    InvalidReqId,
}

impl From<FreeTunnelError> for ProgramError {
    fn from(e: FreeTunnelError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
