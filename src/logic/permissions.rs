use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    constants::{Constants, EthAddress},
    error::FreeTunnelError,
    state::{BasicStorage, ExecutorsInfo},
    utils::{DataAccountUtils, SignatureUtils},
};

pub struct Permissions;

impl Permissions {
    pub(crate) fn assert_only_admin(
        data_account_basic_storage: &AccountInfo,
        account_admin: &AccountInfo,
    ) -> ProgramResult {
        let basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if &basic_storage.admin != account_admin.key {
            Err(FreeTunnelError::NotAdmin.into())
        } else if !account_admin.is_signer {
            Err(FreeTunnelError::AdminNotSigner.into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn assert_only_proposer(
        data_account_basic_storage: &AccountInfo,
        account_proposer: &AccountInfo,
    ) -> ProgramResult {
        let basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if !basic_storage.proposers.contains(account_proposer.key) {
            Err(FreeTunnelError::NotProposer.into())
        } else if !account_proposer.is_signer {
            Err(FreeTunnelError::ProposerNotSigner.into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn add_proposer_internal(
        data_account_basic_storage: &AccountInfo,
        proposer: &Pubkey,
    ) -> ProgramResult {
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if basic_storage.proposers.contains(&proposer) {
            Err(FreeTunnelError::AlreadyProposer.into())
        } else {
            basic_storage.proposers.push(proposer.clone());
            DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)
        }
    }

    pub(crate) fn remove_proposer_internal(
        data_account_basic_storage: &AccountInfo,
        proposer: &Pubkey,
    ) -> ProgramResult {
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        if !basic_storage.proposers.contains(proposer) {
            Err(FreeTunnelError::NotExistingProposer.into())
        } else {
            basic_storage.proposers.retain(|p| p != proposer);
            DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)
        }
    }

    pub(crate) fn init_executors_internal<'a>(
        program_id: &Pubkey,
        system_program: &AccountInfo<'a>,
        account_admin: &AccountInfo<'a>,
        data_account_basic_storage: &AccountInfo,
        data_account_executors_at_index: &AccountInfo<'a>,
        executors: &Vec<EthAddress>,
        threshold: u64,
        exe_index: u64,
    ) -> ProgramResult {
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        Self::assert_only_admin(data_account_basic_storage, account_admin)?;

        // Check conditions
        if threshold > executors.len() as u64 {
            Err(FreeTunnelError::NotMeetThreshold.into())
        } else if basic_storage.executors_group_length != 0 {
            Err(FreeTunnelError::ExecutorsAlreadyInitialized.into())
        } else if threshold == 0 {
            Err(FreeTunnelError::ThresholdMustBeGreaterThanZero.into())
        } else {
            basic_storage.executors_group_length = exe_index + 1;
            SignatureUtils::check_executors_not_duplicated(executors)?;
            DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)?;

            // Write executors data
            DataAccountUtils::create_data_account(
                program_id,
                system_program,
                account_admin,
                data_account_executors_at_index,
                Constants::PREFIX_EXECUTORS,
                &exe_index.to_le_bytes(),
                Constants::SIZE_EXECUTORS_STORAGE + Constants::SIZE_LENGTH,
            ExecutorsInfo {
                index: exe_index,
                threshold,
                active_since: 1,
                inactive_after: 0,
                executors: executors.clone(),
            },
            )
        }
    }

    pub(crate) fn update_executors(
        data_account_basic_storage: &AccountInfo,
        data_account_executors: &AccountInfo,
        data_account_new_executors: &AccountInfo,
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
            + (12 + SignatureUtils::log10(threshold) as usize + 1)
            + (15 + 10)
            + (25 + SignatureUtils::log10(exe_index) as usize + 1);
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

        // Check multi signatures
        SignatureUtils::check_multi_signatures(
            data_account_executors,
            &msg,
            signatures,
            executors,
        )?;

        // Update current executors' inactive_after
        let mut current_executors_info: ExecutorsInfo =
            DataAccountUtils::read_account_data(data_account_executors)?;
        current_executors_info.inactive_after = active_since;
        DataAccountUtils::write_account_data(data_account_executors, current_executors_info)?;

        // Add executors to storage
        let mut basic_storage: BasicStorage =
            DataAccountUtils::read_account_data(data_account_basic_storage)?;
        let new_index = exe_index + 1;
        if new_index == basic_storage.executors_group_length {
            basic_storage.executors_group_length = new_index + 1;
            DataAccountUtils::write_account_data(data_account_basic_storage, basic_storage)?;
            DataAccountUtils::write_account_data(
                data_account_new_executors,
                ExecutorsInfo {
                    index: new_index,
                    threshold,
                    active_since,
                    inactive_after: 0,
                    executors: new_executors.clone(),
                },
            )
        } else {
            let ExecutorsInfo {
                index: _,
                threshold: next_threshold,
                active_since: next_active_since,
                executors: next_executors,
                ..
            } = DataAccountUtils::read_account_data(data_account_new_executors)?;
            if active_since < next_active_since
                || threshold < next_threshold
                || !SignatureUtils::cmp_addr_list(new_executors, &next_executors)
            {
                return Err(FreeTunnelError::FailedToOverwriteExistingExecutors.into());
            }
            DataAccountUtils::write_account_data(
                data_account_new_executors,
                ExecutorsInfo {
                    index: new_index,
                    threshold,
                    active_since,
                    inactive_after: 0,
                    executors: new_executors.clone(),
                },
            )
        }
    }
}
