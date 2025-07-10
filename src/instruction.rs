use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FreeTunnelInstruction {
    Initialize,
    TransferAdmin,
    AddProposer,
    RemoveProposer,
    UpdateExecutors,

    AddToken,
    RemoveToken,

    ProposeMint,
    ProposeMintForBurn,
    ExecuteMint,
    CancelMint,
    ProposeBurn,
    ProposeBurnForMint,
    ExecuteBurn,
    CancelBurn,

    ProposeLock,
    ExecuteLock,
    CancelLock,
    ProposeUnlock,
    ExecuteUnlock,
    CancelUnlock,
}
