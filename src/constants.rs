use solana_program::pubkey::Pubkey;

pub struct Constants;
pub type EthAddress = [u8; 20];

impl Constants {
    // Limits
    pub const MAX_PROPOSERS: usize = 32;
    pub const MAX_EXECUTORS: usize = 32;
    pub const MAX_TOKENS: usize = 32;

    // Zero address and placeholder
    pub const ETH_ZERO_ADDRESS: EthAddress = [0; 20];
    pub const EXECUTED_PLACEHOLDER: Pubkey = Pubkey::new_from_array([0xed; 32]);

    // Contract signer
    pub const CONTRACT_SIGNER: &'static [u8] = b"contract-signer";

    // Bridge related
    pub const HUB_ID: u8 = 0xa1;
    pub const BRIDGE_CHANNEL: &'static [u8] = b"SolvBTC Bridge";
    pub const PROPOSE_PERIOD: u64 = 48 * 60 * 60;
    pub const EXPIRE_PERIOD: u64 = 72 * 60 * 60;
    pub const EXPIRE_EXTRA_PERIOD: u64 = 96 * 60 * 60;
    pub const ETH_SIGN_HEADER: &'static [u8] = b"\x19Ethereum Signed Message:\n";

    // Data account storage location
    pub const BASIC_STORAGE: &'static [u8] = b"basic-storage";
    pub const PREFIX_EXECUTORS: &'static [u8] = b"executors";
    pub const PREFIX_MINT: &'static [u8] = b"mint";
    pub const PREFIX_BURN: &'static [u8] = b"burn";
    pub const PREFIX_LOCK: &'static [u8] = b"lock";
    pub const PREFIX_UNLOCK: &'static [u8] = b"unlock";

    // Data account size
    pub const SIZE_LENGTH: usize = 4; // actual length for the data account (not capacity)
    pub const SIZE_BASIC_STORAGE: usize =
        1 + 32 + (4 + 32 * Self::MAX_PROPOSERS) + 8
        + (4 + Self::MAX_TOKENS * (1 + 32))
        + (4 + Self::MAX_TOKENS * (1 + 32))
        + (4 + Self::MAX_TOKENS * (1 + 1))
        + (4 + Self::MAX_TOKENS * (1 + 8));
    pub const SIZE_EXECUTORS_STORAGE: usize =
        8 + 8 + 8 + 8 + (4 + 20 * Self::MAX_EXECUTORS);
    pub const SIZE_ADDRESS_STORAGE: usize = 32;
}
