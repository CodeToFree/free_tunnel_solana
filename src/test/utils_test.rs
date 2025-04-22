#[cfg(test)]
mod utils_test {

    use crate::utils::SignatureUtils;
    use hex;

    #[test]
    fn test_eth_address_from_pubkey() {
        let pk_hex = "5139c6f948e38d3ffa36df836016aea08f37a940a91323f2a785d17be4353e382b488d0c543c505ec40046afbb2543ba6bb56ca4e26dc6abee13e9add6b7e189";
        let pk: [u8; 64] = hex::decode(pk_hex).unwrap().try_into().unwrap();
        let eth_address = SignatureUtils::eth_address_from_pubkey(pk);
        let eth_address_expected_hex = "052c7707093534035fc2ed60de35e11bebb6486b";
        let eth_address_expected: [u8; 20] = hex::decode(eth_address_expected_hex)
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(eth_address, eth_address_expected);
    }

    #[test]
    fn test_recover_eth_address() {
        let message = b"stupid";
        let signature_hex = "6fd862958c41d532022e404a809e92ec699bd0739f8d782ca752b07ff978f341f43065a96dc53a21b4eb4ce96a84a7c4103e3485b0c87d868df545fcce0f3983";
        let signature: [u8; 64] = hex::decode(signature_hex).unwrap().try_into().unwrap();
        let eth_address = SignatureUtils::recover_eth_address(message, signature);
        let eth_address_expected_hex = "2eF8a51F8fF129DBb874A0efB021702F59C1b211";
        let eth_address_expected: [u8; 20] = hex::decode(eth_address_expected_hex)
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(eth_address, eth_address_expected);
    }
}
