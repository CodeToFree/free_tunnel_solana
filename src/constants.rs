pub struct Constants;
pub type EthAddress = [u8; 20];

impl Constants {
    pub const ETH_ZERO_ADDRESS: EthAddress = [0; 20];

    pub const PREFIX_MINT: &'static [u8] = b"mint";
    pub const PREFIX_BURN: &'static [u8] = b"burn";
    pub const PREFIX_LOCK: &'static [u8] = b"lock";
    pub const PREFIX_UNLOCK: &'static [u8] = b"unlock";
    
}
