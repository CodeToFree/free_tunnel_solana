use borsh::{BorshDeserialize, BorshSerialize};

use crate::constants::EthAddress;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FreeTunnelInstruction {
    // The admin(deployer) must call this init function first
    /// [0]        
    /// 0. account_payer: the account pays for the data accounts creation
    /// 1. account_admin: the admin account, should be signer
    /// 2. data_account_basic_storage: data account for storing basic storage
    /// 3. data_account_tokens_proposers: data account for storing tokens and proposers
    /// 4. data_account_executors_at_index: data account for storing executors at index
    Initialize {
        is_mint_contract: bool,
        executors: Vec<EthAddress>,
        threshold: u64,
        exe_index: u64,
    },

    /// [1] Transfer admin
    /// 0. account_admin
    /// 1. data_account_basic_storage
    TransferAdmin {
        new_admin: EthAddress,
    },

    /// [2]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    AddProposer,
    
    /// [3]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    RemoveProposer,

    /// [4]
    /// 0. data_account_basic_storage
    /// 1. data_account_current_executors: data account for storing executors at `index`
    /// 2. data_account_next_executors: data account for storing executors at `index + 1`
    UpdateExecutors,

    /// [5]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    AddToken,

    /// [6]
    /// 0. account_admin
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    RemoveToken,


    /// [7]
    /// 0. account_payer
    /// 1. account_proposer
    /// 2. data_account_basic_storage
    /// 3. data_account_tokens_proposers
    /// 4. data_account_proposed_mint: data account for storing `ProposedMint` (recipient)
    ProposeMint,

    /// [8]
    /// 0. account_payer
    /// 1. account_proposer
    /// 2. data_account_basic_storage
    /// 3. data_account_tokens_proposers
    /// 4. data_account_proposed_mint
    ProposeMintForBurn,

    /// [9]
    /// 0. system_account_token_program: token program account, should be `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` on mainnet
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    /// 3. data_account_proposed_mint
    /// 4. data_account_current_executors
    /// 5. data_account_next_executors
    /// 6. token_account_recipient: token account for the recipient, should be different for each token
    /// 7. account_token_mint: token mint account (token contract address)
    /// 8. account_multisig_owner: multisig owner account
    /// 9..n account_multisig_wallets: multisig wallets accounts
    ExecuteMint,
    
    /// [10]
    /// 0. data_account_basic_storage
    /// 1. data_account_proposed_mint
    CancelMint,
    
    /// [11]
    /// 0. account_payer
    /// 1. account_proposer
    /// 2. system_account_token_program
    /// 3. data_account_basic_storage
    /// 4. data_account_tokens_proposers
    /// 5. data_account_proposed_burn: data account for storing `ProposedBurn` (recipient)
    /// 6. token_account_proposer: token account for the proposer, should be different for each token
    /// 7. token_account_contract: token account for this contract, should be different for each token
    ProposeBurn,
    
    /// [12]
    /// 0. account_payer
    /// 1. account_proposer
    /// 2. system_account_token_program
    /// 3. data_account_basic_storage
    /// 4. data_account_tokens_proposers
    /// 5. data_account_proposed_burn
    /// 6. token_account_proposer
    /// 7. token_account_contract
    ProposeBurnForMint,
    
    /// [13]
    /// 0. system_account_token_program
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    /// 3. data_account_proposed_burn
    /// 4. data_account_current_executors
    /// 5. data_account_next_executors
    /// 6. token_account_contract
    /// 7. account_contract_signer: contract signer that can sign for the token transfer
    /// 8. account_token_mint
    ExecuteBurn,
    
    /// [14]
    /// 0. system_account_token_program
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    /// 3. data_account_proposed_burn
    /// 4. token_account_proposer
    /// 5. token_account_contract
    /// 6. account_contract_signer
    CancelBurn,

    
    /// [15]
    /// 0. account_payer
    /// 1. account_proposer
    /// 2. system_account_token_program
    /// 3. data_account_basic_storage
    /// 4. data_account_tokens_proposers
    /// 5. data_account_proposed_lock
    /// 6. token_account_proposer
    /// 7. token_account_contract
    ProposeLock,
    
    /// [16]
    /// 0. data_account_basic_storage
    /// 1. data_account_tokens_proposers
    /// 2. data_account_proposed_lock
    /// 3. data_account_current_executors
    /// 4. data_account_next_executors
    ExecuteLock,
    
    /// [17]
    /// 0. system_account_token_program
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    /// 3. data_account_proposed_lock
    /// 4. token_account_proposer
    /// 5. token_account_contract
    /// 6. account_contract_signer
    CancelLock,
    
    /// [18]
    /// 0. account_payer
    /// 1. account_proposer
    /// 2. data_account_basic_storage
    /// 3. data_account_tokens_proposers
    /// 4. data_account_proposed_unlock
    ProposeUnlock,
    
    /// [19]
    /// 0. system_account_token_program
    /// 1. data_account_basic_storage
    /// 2. data_account_tokens_proposers
    /// 3. data_account_proposed_unlock
    /// 4. data_account_current_executors
    /// 5. data_account_next_executors
    /// 6. token_account_recipient
    /// 7. token_account_contract
    /// 8. account_contract_signer
    ExecuteUnlock,
    
    /// [20]
    /// 0. data_account_basic_storage
    /// 1. data_account_tokens_proposers
    /// 2. data_account_proposed_unlock
    CancelUnlock,
}
