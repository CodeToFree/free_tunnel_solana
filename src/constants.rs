pub struct Constants;
pub type EthAddress = [u8; 20];

impl Constants {
    pub const ETH_ZERO_ADDRESS: EthAddress = [0; 20];

    // Bridge related
    pub const BRIDGE_CHANNEL: &'static [u8] = b"Solana Bridge";
    pub const PROPOSE_PERIOD: u64 = 48 * 60 * 60;
    pub const EXPIRE_PERIOD: u64 = 72 * 60 * 60;
    pub const EXPIRE_EXTRA_PERIOD: u64 = 96 * 60 * 60;
    pub const ETH_SIGN_HEADER: &'static [u8] = b"\x19Ethereum Signed Message:\n";

    // Data account storage location
    pub const BASIC_STORAGE: &'static [u8] = b"basic-storage";
    pub const TOKENS_PROPOSERS: &'static [u8] = b"tokens-proposers";
    pub const PREFIX_EXECUTORS: &'static [u8] = b"executors";
    pub const PREFIX_MINT: &'static [u8] = b"mint";
    pub const PREFIX_BURN: &'static [u8] = b"burn";
    pub const PREFIX_LOCK: &'static [u8] = b"lock";
    pub const PREFIX_UNLOCK: &'static [u8] = b"unlock";

    // Data account size
    pub const SIZE_BASIC_STORAGE: usize = 32 + 8;     // admin and executors_group_length
    pub const SIZE_TOKENS_PROPOSERS: usize = 32 * 256 + 32 * 256;       // tokens and proposers
    pub const SIZE_EXECUTORS_STORAGE: usize = 2 + 1 + 8 + 20 * 256;     // index, threshold, active_since and executors
    pub const SIZE_ADDRESS_STORAGE: usize = 32;
}
