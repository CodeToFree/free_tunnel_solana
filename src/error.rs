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
    // Solana-only account/token checks
    InvalidSystemProgram = 0,
    InvalidTokenProgram = 1,
    InvalidTokenMint = 2,
    InvalidTokenAccount = 3,
    ContractSignerMismatch = 4,
    ArithmeticOverflow = 5,
    RequireSigner = 6,
    StorageLimitReached = 7,

    // Solana-only mint/lock checks
    NotMintContract = 8,
    NotLockContract = 9,

    // Req Helpers (aligned with Aptos)
    TokenIndexOccupied = 10,
    TokenIndexCannotBeZero = 11,
    TokenIndexNonExistent = 12,
    NotMintSide = 14,
    NotMintOppositeSide = 15,
    CreatedTimeTooEarly = 16,
    CreatedTimeTooLate = 17,
    AmountCannotBeZero = 18,
    TokenMismatch = 19,

    // Permissions & Signature (aligned with Aptos)
    RequireAdminSigner = 20,
    RequireProposerSigner = 21,
    AlreadyProposer = 22,
    NotExistingProposer = 23,
    ExecutorsAlreadyInitialized = 24,
    ThresholdMustBeGreaterThanZero = 25,
    ArrayLengthNotEqual = 26,
    NotMeetThreshold = 27,
    ExecutorsNotYetActive = 28,
    ExecutorsOfNextIndexIsActive = 29,
    DuplicatedExecutors = 30,
    NonExecutors = 31,
    SignerCannotBeZeroAddress = 32,
    InvalidSignature = 34,
    ActiveSinceShouldAfter36h = 35,
    ActiveSinceShouldWithin5d = 36,
    FailedToOverwriteExistingExecutors = 37,

    LockedBalanceMustBeZero = 40,
    LockedBalanceInsufficient = 41,
    RefundAccountNotWritable = 42,

    // Mint/Lock (aligned with Aptos)
    ReqIdOccupied = 50,
    NotLockMint = 51,
    NotBurnUnlock = 52,
    NotBurnMint = 53,
    InvalidProposer = 54,
    InvalidRecipient = 55,
    WaitUntilExpired = 56,
    ReqIdExecuted = 57,
}

impl From<FreeTunnelError> for ProgramError {
    fn from(e: FreeTunnelError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
