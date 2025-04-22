use solana_program::{keccak, secp256k1_recover::secp256k1_recover};

pub struct TypeUtils;

impl TypeUtils {

    pub fn eth_address_from_pubkey(pk: [u8; 64]) -> [u8; 20] {
        let hash = keccak::hash(&pk).to_bytes();
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..32]);
        address
    }

    pub fn recover_eth_address(message: &[u8], signature: [u8; 64]) -> [u8; 20] {
        let digest = keccak::hash(&message).to_bytes();

        let mut signature_split = [0 as u8; 64];
        signature_split.copy_from_slice(&signature);
        let first_bit_of_s = signature_split.get_mut(32).unwrap();
        let recovery_id = *first_bit_of_s >> 7;
        *first_bit_of_s = *first_bit_of_s & 0x7f;

        let pubkey = secp256k1_recover(&digest, recovery_id, &signature_split);
        match pubkey {
            Ok(eth_pubkey) => Self::eth_address_from_pubkey(eth_pubkey.to_bytes()),
            Err(_error) => [0; 20]
        }
    }
}


// /**
//  * smallU64ToString
//  * smallU64Log10
//  * hexToString
//  * ethAddressFromPubkey
//  * assertEthAddressList
//  */