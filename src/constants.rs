use solana_program::pubkey::Pubkey;

pub struct Constants;
pub type EthAddress = [u8; 20];

impl Constants {
    // Zero address and placeholder
    pub const ETH_ZERO_ADDRESS: EthAddress = [0; 20];
    pub const EXECUTED_PLACEHOLDER: Pubkey = Pubkey::new_from_array([0xed; 32]);

    // Contract signer
    pub const CONTRACT_SIGNER: &'static [u8] = b"contract-signer";

    // Bridge related
    pub const CHAIN: u8 = 0xff;
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
    pub const SIZE_LENGTH: usize = 4; // actual length for the data account (not capacity)
    pub const SIZE_BASIC_STORAGE: usize = 1 + 32 + 8; // `mint_or_lock`, `admin` and `executors_group_length`
    pub const SIZE_TOKENS_PROPOSERS: usize = 32 * (32 + 1 + 8 + 32); // `tokens` (and `decimals`) and `proposers`, up to 32 tokens and 32 proposers
    pub const SIZE_EXECUTORS_STORAGE: usize = 8 + 8 + 8 + 20 * 64; // `index`, `threshold`, `active_since` and `executors`, up to 64 executors
    pub const SIZE_ADDRESS_STORAGE: usize = 32;
}
