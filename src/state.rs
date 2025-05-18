use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::constants::EthAddress;

// struct ReqHelpersStorage has key {
//     ✅ tokens: table::Table<u8, Object<Metadata>>,
// }

// pub struct PermissionsStorage {
//     admin: Pubkey,
//     ✅ _proposerIndex: table::Table<address, u64>,
//     ✅ _proposerList: vector<address>,
//     ✅ _executorsForIndex: vector<vector<vector<u8>>>,
//     ✅ _exeThresholdForIndex: vector<u64>,
//     ✅ _exeActiveSinceForIndex: vector<u64>,
// }

// struct AtomicMintStorage has key, store {
//     ❎ store_contract_signer_extend_ref: ExtendRef,
//     proposedMint: table::Table<vector<u8>, address>,
//     proposedBurn: table::Table<vector<u8>, address>,
// }

// struct AtomicLockStorage has key, store {
//     ❎ store_contract_signer_extend_ref: ExtendRef,
//     proposedLock: table::Table<vector<u8>, address>,
//     proposedUnlock: table::Table<vector<u8>, address>,
//     lockedBalanceOf: table::Table<u8, u64>,
// }


#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct SupportedTokens {
    data: [Pubkey; 256],        // support up to 256 tokens
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Proposers {
    data: [Pubkey; 256],        // support up to 256 proposers
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Executors {
    index: u16,
    threshold: u16,
    active_since: u64,
    members: [EthAddress; 256],
}

