#[cfg(test)]
mod req_helpers_test {

    use crate::logic::req_helpers::ReqId;
    use hex;

    #[test]
    fn test_decoding_reqid() {
        let req_id_u8: [u8; 32] =
            hex::decode("112233445566778899aabbccddeeff00ffffffffffffffffffffffffffffffff")
                .unwrap()
                .try_into()
                .unwrap();
        let req_id = ReqId::new(req_id_u8);
        assert_eq!(req_id.version(), 0x11);
        assert_eq!(req_id.created_time(), 0x2233445566);
        assert_eq!(req_id.action(), 0x77);
        assert_eq!(req_id.token_index(), 0x88);
        assert_eq!(req_id.raw_amount(), 0x99aabbccddeeff00);
        assert_eq!(req_id.assert_from_chain_only(), Ok(()));
        assert_eq!(req_id.assert_to_chain_only(), Ok(()));
    }

    #[test]
    fn test_msg_from_req_signing_message_1() {
        // action 1: lock-mint
        let req_id_u8: [u8; 32] =
            hex::decode("112233445566018899aabbccddeeff004040ffffffffffffffffffffffffffff")
                .unwrap()
                .try_into()
                .unwrap();
        let req_id = ReqId::new(req_id_u8);
        let msg = req_id.msg_from_req_signing_message();
        let expected =
            String::from("\x19Ethereum Signed Message:\n111[Solana Bridge]\nSign to execute a ")
                + "lock-mint:\n0x112233445566018899aabbccddeeff004040ffffffffffffffffffffffffffff";
        assert_eq!(msg, expected.as_bytes());
    }

    #[test]
    fn test_msg_from_req_signing_message_2() {
        // action 2: burn-unlock
        let req_id_u8: [u8; 32] =
            hex::decode("112233445566028899aabbccddeeff004040ffffffffffffffffffffffffffff")
                .unwrap()
                .try_into()
                .unwrap();
        let req_id = ReqId::new(req_id_u8);
        let msg = req_id.msg_from_req_signing_message();
        let expected = String::from(
            "\x19Ethereum Signed Message:\n113[Solana Bridge]\nSign to execute a ",
        )
            + "burn-unlock:\n0x112233445566028899aabbccddeeff004040ffffffffffffffffffffffffffff";
        assert_eq!(msg, expected.as_bytes());
    }

    #[test]
    fn test_msg_from_req_signing_message_3() {
        // action 3: burn-mint
        let req_id_u8: [u8; 32] =
            hex::decode("112233445566038899aabbccddeeff004040ffffffffffffffffffffffffffff")
                .unwrap()
                .try_into()
                .unwrap();
        let req_id = ReqId::new(req_id_u8);
        let msg = req_id.msg_from_req_signing_message();
        let expected =
            String::from("\x19Ethereum Signed Message:\n111[Solana Bridge]\nSign to execute a ")
                + "burn-mint:\n0x112233445566038899aabbccddeeff004040ffffffffffffffffffffffffffff";
        assert_eq!(msg, expected.as_bytes());
    }

    #[test]
    fn test_msg_from_req_signing_message_4() {
        // action 4: invalid
        let req_id_u8: [u8; 32] =
            hex::decode("112233445566048899aabbccddeeff004040ffffffffffffffffffffffffffff")
                .unwrap()
                .try_into()
                .unwrap();
        let req_id = ReqId::new(req_id_u8);
        let msg = req_id.msg_from_req_signing_message();
        assert_eq!(msg, vec![] as Vec<u8>);
    }
}
