use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::constants::EthAddress;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BasicStorage {
    pub admin: Pubkey,
    pub executors_group_length: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TokensAndProposers {
    pub tokens: [Pubkey; 256],    // support up to 256 tokens
    pub proposers: [Pubkey; 256], // support up to 256 proposers
    pub decimals: [u8; 256],      // decimals of each token
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutorsInfo {
    pub index: u16,
    pub threshold: u8,
    pub active_since: u64,
    pub executors: Vec<EthAddress>,
}
