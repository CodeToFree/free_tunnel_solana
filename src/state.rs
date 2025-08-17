use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::constants::EthAddress;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BasicStorage {
    pub mint_or_lock: bool, // true for mint, false for lock
    pub admin: Pubkey,
    pub executors_group_length: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TokensAndProposers {
    pub tokens: [Pubkey; 256], // support up to 256 tokens, structured as mapping
    pub decimals: [u8; 256],   // decimals of each token
    pub locked_balance: [u64; 256], // locked balance of each token
    pub proposers: Vec<Pubkey>, // support up to 256 proposers, structured as list
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutorsInfo {
    pub index: u64,
    pub threshold: u64,
    pub active_since: u64,
    pub executors: Vec<EthAddress>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProposedLock {
    pub inner: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProposedMint {
    pub inner: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProposedBurn {
    pub inner: Pubkey,
}
