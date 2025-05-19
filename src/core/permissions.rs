use borsh::{BorshDeserialize, BorshSerialize};
use std::{cmp::Ordering, collections::HashSet};

use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    keccak,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    secp256k1_recover::secp256k1_recover,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

use crate::constants::{Constants, EthAddress};
use crate::error::{DataAccountError, FreeTunnelError};
use crate::state::{BasicStorage, ExecutorsInfo};
use crate::utils::SignatureUtils;

pub struct Permissions;

impl Permissions {
    fn log10(n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            (n as f64).log10().floor() as u64
        }
    }

    fn update_executors(
        data_account_basic_storage: &AccountInfo,
        data_account_current_executors: &AccountInfo,
        data_account_next_executors: &AccountInfo,
        new_executors: &Vec<EthAddress>,
        threshold: u64,
        active_since: u64,
        signatures: &Vec<[u8; 64]>,
        executors: &Vec<EthAddress>,
        exe_index: u64,
    ) -> ProgramResult {
        let now = Clock::get()?.unix_timestamp;

        // Check threshold and active since
        if threshold == 0 {
            return Err(FreeTunnelError::ThresholdMustBeGreaterThanZero.into());
        }
        if threshold > new_executors.len() as u64 {
            return Err(FreeTunnelError::NotMeetThreshold.into());
        }
        if (active_since as i64) < now + 36 * 3600 {
            return Err(FreeTunnelError::ActiveSinceShouldAfter36h.into());
        }
        if (active_since as i64) > now + 120 * 3600 {
            return Err(FreeTunnelError::ActiveSinceShouldWithin5d.into());
        }
        SignatureUtils::check_executors_not_duplicated(new_executors)?;

        // Construct message
        let mut msg = Constants::ETH_SIGN_HEADER.to_vec();
        let length = 3
            + Constants::BRIDGE_CHANNEL.len()
            + (29 + 43 * new_executors.len())
            + (12 + Self::log10(threshold) as usize + 1)
            + (15 + 10)
            + (25 + Self::log10(exe_index) as usize + 1);
        msg.extend_from_slice(length.to_string().as_bytes());
        msg.extend_from_slice(b"[");
        msg.extend_from_slice(Constants::BRIDGE_CHANNEL);
        msg.extend_from_slice(b"]\n");
        msg.extend_from_slice(b"Sign to update executors to:\n");
        msg.extend_from_slice(&SignatureUtils::join_address_list(new_executors));
        msg.extend_from_slice(b"Threshold: ");
        msg.extend_from_slice(threshold.to_string().as_bytes());
        msg.extend_from_slice(b"\n");
        msg.extend_from_slice(b"Active since: ");
        msg.extend_from_slice(active_since.to_string().as_bytes());
        msg.extend_from_slice(b"\n");
        msg.extend_from_slice(b"Current executors index: ");
        msg.extend_from_slice(exe_index.to_string().as_bytes());

        SignatureUtils::check_multi_signatures(
            data_account_basic_storage,
            data_account_current_executors,
            data_account_next_executors,
            &msg,
            signatures,
            executors,
            exe_index,
        )?;
        
        
        

        Ok(())
    }

    // ) acquires PermissionsStorage {
    //     let storeP = borrow_global_mut<PermissionsStorage>(@free_tunnel_aptos);
    //     let newIndex = exeIndex + 1;
    //     if (newIndex == vector::length(&storeP._exeActiveSinceForIndex)) {
    //         vector::push_back(&mut storeP._executorsForIndex, newExecutors);
    //         vector::push_back(&mut storeP._exeThresholdForIndex, threshold);
    //         vector::push_back(&mut storeP._exeActiveSinceForIndex, activeSince);
    //     } else {
    //         assert!(
    //             activeSince >= *vector::borrow(&storeP._exeActiveSinceForIndex, newIndex),
    //             EFAILED_TO_OVERWRITE_EXISTING_EXECUTORS
    //         );
    //         assert!(
    //             threshold >= *vector::borrow(&storeP._exeThresholdForIndex, newIndex),
    //             EFAILED_TO_OVERWRITE_EXISTING_EXECUTORS
    //         );
    //         assert!(
    //             cmpAddrList(newExecutors, *vector::borrow(&storeP._executorsForIndex, newIndex)),
    //             EFAILED_TO_OVERWRITE_EXISTING_EXECUTORS
    //         );
    //         *vector::borrow_mut(&mut storeP._executorsForIndex, newIndex) = newExecutors;
    //         *vector::borrow_mut(&mut storeP._exeThresholdForIndex, newIndex) = threshold;
    //         *vector::borrow_mut(&mut storeP._exeActiveSinceForIndex, newIndex) = activeSince;
    //     };
    //     event::emit(ExecutorsUpdated { executors: newExecutors, threshold, activeSince, exeIndex: newIndex });
    // }
}
