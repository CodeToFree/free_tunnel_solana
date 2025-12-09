use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::{constants::EthAddress, logic::req_helpers::ReqId};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FreeTunnelInstruction {
    // The admin(deployer) must call this init function first
    /// [0]        
    /// [0]        
    /// 0. system_program: system program account, `11111111111111111111111111111111`
    /// 1. account_admin: the admin account, should be signer and payer
    /// 2. data_account_basic_storage: data account for storing basic storage (includes tokens, decimals, locked_balance, and proposers)
    /// 3. data_account_executors_at_index: data account for storing executors at index
    Initialize {
        is_mint_contract: bool,
        executors: Vec<EthAddress>,
        threshold: u64,
        exe_index: u64,
    },

    /// [1] Transfer admin
    /// 0. account_admin
    /// 1. data_account_basic_storage
    TransferAdmin { new_admin: Pubkey },

    /// [2]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    AddProposer { new_proposer: Pubkey },

    /// [3]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    RemoveProposer { proposer: Pubkey },

    /// [4]
    /// 0. data_account_basic_storage
    /// 1. data_account_executors: data account for storing executors at `index`
    /// 2. data_account_new_executors: data account for storing executors at `index + 1`
    UpdateExecutors {
        new_executors: Vec<EthAddress>,
        threshold: u64,
        active_since: u64,
        signatures: Vec<[u8; 64]>,
        executors: Vec<EthAddress>,
        exe_index: u64,
    },

    /// [5]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    AddToken {
        token_index: u8,
        token_pubkey: Pubkey,
        token_decimals: u8,
    },

    /// [6]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    RemoveToken { token_index: u8 },

    /// [7]
    /// 0. system_program
    /// 1. account_proposer: the proposer account, should be signer and payer
    /// 2. data_account_basic_storage
    /// 3. data_account_proposed_mint: data account for storing `ProposedMint` (recipient)
    ProposeMint { req_id: ReqId, recipient: Pubkey },

    /// [8]
    /// 0. system_program
    /// 1. account_proposer: the proposer account, should be signer and payer
    /// 2. data_account_basic_storage
    /// 3. data_account_proposed_mint
    ProposeMintForBurn { req_id: ReqId, recipient: Pubkey },

    /// [9]
    /// 0. system_account_token_program: token program account, should be `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` on mainnet
    /// 1. account_contract_signer: contract signer that can sign for the token transfer
    /// 2. token_account_recipient: token account for the recipient, should be different for each token
    /// 3. data_account_basic_storage
    /// 4. data_account_proposed_mint
    /// 5. data_account_executors
    /// 6. account_token_mint: token mint account (token contract address)
    /// 7. account_multisig_owner: multisig owner account
    ExecuteMint {
        req_id: ReqId,
        signatures: Vec<[u8; 64]>,
        executors: Vec<EthAddress>,
        exe_index: u64,
    },

    /// [10]
    /// 0. data_account_basic_storage
    /// 1. data_account_proposed_mint
    CancelMint { req_id: ReqId },

    /// [11]
    /// 0. system_program
    /// 1. system_account_token_program
    /// 2. account_proposer: the proposer account, should be signer and payer
    /// 3. token_account_contract: token account for this contract, should be different for each token
    /// 4. token_account_proposer: token account for the proposer, should be different for each token
    /// 5. data_account_basic_storage
    /// 6. data_account_proposed_burn: data account for storing `ProposedBurn` (recipient)
    ProposeBurn { req_id: ReqId },

    /// [12]
    /// 0. system_program
    /// 1. system_account_token_program
    /// 2. account_proposer: the proposer account, should be signer and payer
    /// 3. token_account_contract
    /// 4. token_account_proposer
    /// 5. data_account_basic_storage
    /// 6. data_account_proposed_burn
    ProposeBurnForMint { req_id: ReqId },

    /// [13]
    /// 0. system_account_token_program
    /// 1. account_contract_signer: contract signer that can sign for the token transfer
    /// 2. token_account_contract
    /// 3. data_account_basic_storage
    /// 4. data_account_proposed_burn
    /// 5. data_account_executors
    /// 6. account_token_mint
    ExecuteBurn {
        req_id: ReqId,
        signatures: Vec<[u8; 64]>,
        executors: Vec<EthAddress>,
        exe_index: u64,
    },

    /// [14]
    /// 0. system_account_token_program
    /// 1. account_contract_signer
    /// 2. token_account_contract
    /// 3. token_account_proposer
    /// 4. data_account_basic_storage
    /// 5. data_account_proposed_burn
    CancelBurn { req_id: ReqId },

    /// [15]
    /// 0. system_program
    /// 1. system_account_token_program
    /// 2. account_proposer: the proposer account, should be signer and payer
    /// 3. token_account_contract
    /// 4. token_account_proposer
    /// 5. data_account_basic_storage
    /// 6. data_account_proposed_lock
    ProposeLock { req_id: ReqId },

    /// [16]
    /// 0. data_account_basic_storage
    /// 1. data_account_proposed_lock
    /// 2. data_account_executors
    ExecuteLock {
        req_id: ReqId,
        signatures: Vec<[u8; 64]>,
        executors: Vec<EthAddress>,
        exe_index: u64,
    },

    /// [17]
    /// 0. system_account_token_program
    /// 1. account_contract_signer
    /// 2. token_account_contract
    /// 3. token_account_proposer
    /// 4. data_account_basic_storage
    /// 5. data_account_proposed_lock
    CancelLock { req_id: ReqId },

    /// [18]
    /// 0. system_program
    /// 1. account_proposer: the proposer account, should be signer and payer
    /// 2. data_account_basic_storage
    /// 3. data_account_proposed_unlock
    ProposeUnlock { req_id: ReqId, recipient: Pubkey },

    /// [19]
    /// 0. system_account_token_program
    /// 1. account_contract_signer
    /// 2. token_account_contract
    /// 3. token_account_recipient
    /// 4. data_account_basic_storage
    /// 5. data_account_proposed_unlock
    /// 6. data_account_executors
    ExecuteUnlock {
        req_id: ReqId,
        signatures: Vec<[u8; 64]>,
        executors: Vec<EthAddress>,
        exe_index: u64,
    },

    /// [20]
    /// 0. data_account_basic_storage
    /// 1. data_account_proposed_unlock
    CancelUnlock { req_id: ReqId },
}

impl FreeTunnelInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        match variant {
            0 => {
                let (is_mint_contract, executors, threshold, exe_index) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::Initialize {
                    is_mint_contract,
                    executors,
                    threshold,
                    exe_index,
                })
            }
            1 => {
                let new_admin = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::TransferAdmin { new_admin })
            }
            2 => {
                let new_proposer = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::AddProposer { new_proposer })
            }
            3 => {
                let proposer = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::RemoveProposer { proposer })
            }
            4 => {
                let (new_executors, threshold, active_since, signatures, executors, exe_index) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::UpdateExecutors {
                    new_executors,
                    threshold,
                    active_since,
                    signatures,
                    executors,
                    exe_index,
                })
            }
            5 => {
                let (token_index, token_pubkey, token_decimals) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::AddToken {
                    token_index,
                    token_pubkey,
                    token_decimals,
                })
            }
            6 => {
                let token_index = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::RemoveToken { token_index })
            }
            7 => {
                let (req_id, recipient) = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ProposeMint { req_id, recipient })
            }
            8 => {
                let (req_id, recipient) = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ProposeMintForBurn { req_id, recipient })
            }
            9 => {
                let (req_id, signatures, executors, exe_index) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ExecuteMint {
                    req_id,
                    signatures,
                    executors,
                    exe_index,
                })
            }
            10 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::CancelMint { req_id })
            }
            11 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ProposeBurn { req_id })
            }
            12 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ProposeBurnForMint { req_id })
            }
            13 => {
                let (req_id, signatures, executors, exe_index) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ExecuteBurn {
                    req_id,
                    signatures,
                    executors,
                    exe_index,
                })
            }
            14 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::CancelBurn { req_id })
            }
            15 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ProposeLock { req_id })
            }
            16 => {
                let (req_id, signatures, executors, exe_index) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ExecuteLock {
                    req_id,
                    signatures,
                    executors,
                    exe_index,
                })
            }
            17 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::CancelLock { req_id })
            }
            18 => {
                let (req_id, recipient) = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ProposeUnlock { req_id, recipient })
            }
            19 => {
                let (req_id, signatures, executors, exe_index) =
                    BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::ExecuteUnlock {
                    req_id,
                    signatures,
                    executors,
                    exe_index,
                })
            }
            20 => {
                let req_id = BorshDeserialize::try_from_slice(rest)?;
                Ok(Self::CancelUnlock { req_id })
            }
            // If the variant is not one of 0-20, return an error
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
