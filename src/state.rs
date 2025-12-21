use std::ops::{Index, IndexMut};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::{
    constants::{Constants, EthAddress},
    error::FreeTunnelError,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BasicStorage {
    pub mint_or_lock: bool, // true for mint, false for lock
    pub admin: Pubkey,
    pub proposers: Vec<Pubkey>, // support up to MAX_PROPOSERS, structured as list
    pub executors_group_length: u64,
    pub tokens: SparseArray<Pubkey>, // support up MAX_TOKENS tokens
    pub vaults: SparseArray<Pubkey>, // contract ATA per token
    pub decimals: SparseArray<u8>, // decimals of each token
    pub locked_balance: SparseArray<u64>, // locked balance of each token
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ExecutorsInfo {
    pub index: u64,
    pub threshold: u64,
    pub active_since: u64,
    pub inactive_after: u64, // 0 means never inactive
    pub executors: Vec<EthAddress>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProposedLock {
    pub inner: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProposedUnlock {
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

// Implement for `TokensAndProposers`
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct SparseArray<Value> {
    inner: Vec<(u8, Value)>,
}

impl<Value> Default for SparseArray<Value> {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<Value> SparseArray<Value> {
    pub fn insert(&mut self, id: u8, value: Value) -> Result<Option<Value>, ProgramError> {
        match self.inner.binary_search_by_key(&id, |&(k, _)| k) {
            Ok(index) => {
                let old_value = std::mem::replace(&mut self.inner[index].1, value);
                Ok(Some(old_value))
            }
            Err(index) => {
                if self.inner.len() >= Constants::MAX_TOKENS {
                    return Err(FreeTunnelError::StorageLimitReached.into());
                }
                self.inner.insert(index, (id, value));
                Ok(None)
            }
        }
    }

    pub fn remove(&mut self, id: u8) -> Option<Value> {
        match self.inner.binary_search_by_key(&id, |&(k, _)| k) {
            Ok(index) => Some(self.inner.remove(index).1),
            Err(_) => None,
        }
    }

    pub fn get(&self, id: u8) -> Option<&Value> {
        match self.inner.binary_search_by_key(&id, |&(k, _)| k) {
            Ok(index) => Some(&self.inner[index].1),
            Err(_) => None,
        }
    }

    pub fn get_mut(&mut self, id: u8) -> Option<&mut Value> {
        match self.inner.binary_search_by_key(&id, |&(k, _)| k) {
            Ok(index) => Some(&mut self.inner[index].1),
            Err(_) => None,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<Value> Index<u8> for SparseArray<Value> {
    type Output = Value;

    fn index(&self, id: u8) -> &Self::Output {
        self.get(id)
            .expect("SparseArray: no entry found for the given id")
    }
}

impl<Value> IndexMut<u8> for SparseArray<Value> {
    fn index_mut(&mut self, id: u8) -> &mut Self::Output {
        self.get_mut(id)
            .expect("SparseArray: no entry found for the given id")
    }
}
