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
    pub tokens: Vec<Pubkey>,    // support up to 256 tokens
    pub proposers: Vec<Pubkey>, // support up to 256 proposers
    pub decimals: Vec<u8>,      // decimals of each token
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutorsInfo {
    pub index: u64,
    pub threshold: u64,
    pub active_since: u64,
    pub executors: Vec<EthAddress>,
}
