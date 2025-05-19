pub struct ReqId {
    /// In format of: `version:uint8|createdTime:uint40|action:uint8`
    ///     + `tokenIndex:uint8|amount:uint64|from:uint8|to:uint8|(TBD):uint112`
    data: [u8; 32],
}

impl ReqId {
    // public(friend) fun versionFrom(reqId: &vector<u8>): u8 {
    //     *vector::borrow(reqId, 0)
    // }

    // public(friend) fun createdTimeFrom(reqId: &vector<u8>): u64 {
    //     let time = (*vector::borrow(reqId, 1) as u64);
    //     let i = 2;
    //     while (i < 6) {
    //         time = (time << 8) + (*vector::borrow(reqId, i) as u64);
    //         i = i + 1;
    //     };
    //     time
    // }

    // public(friend) fun checkCreatedTimeFrom(reqId: &vector<u8>): u64 {
    //     let time = createdTimeFrom(reqId);
    //     assert!(time > now_seconds() - PROPOSE_PERIOD(), ECREATED_TIME_TOO_EARLY);
    //     assert!(time < now_seconds() + 60, ECREATED_TIME_TOO_LATE);
    //     time
    // }

    // public(friend) fun actionFrom(reqId: &vector<u8>): u8 {
    //     *vector::borrow(reqId, 6)
    // }

    // public(friend) fun decodeTokenIndex(reqId: &vector<u8>): u8 {
    //     *vector::borrow(reqId, 7)
    // }

    // public(friend) fun tokenIndexFrom(reqId: &vector<u8>): u8 acquires ReqHelpersStorage {
    //     let tokenIndex = decodeTokenIndex(reqId);
    //     let storeR = borrow_global_mut<ReqHelpersStorage>(@free_tunnel_aptos);
    //     assert!(table::contains(&storeR.tokens, tokenIndex), ETOKEN_INDEX_NONEXISTENT);
    //     tokenIndex
    // }

    // public(friend) fun tokenMetadataFrom(reqId: &vector<u8>): Object<Metadata> acquires ReqHelpersStorage {
    //     let tokenIndex = decodeTokenIndex(reqId);
    //     let storeR = borrow_global_mut<ReqHelpersStorage>(@free_tunnel_aptos);
    //     assert!(table::contains(&storeR.tokens, tokenIndex), ETOKEN_INDEX_NONEXISTENT);
    //     *table::borrow(&storeR.tokens, tokenIndex)
    // }

    // fun decodeAmount(reqId: &vector<u8>): u64 {
    //     let amount = (*vector::borrow(reqId, 8) as u64);
    //     let i = 9;
    //     while (i < 16) {
    //         amount = (amount << 8) + (*vector::borrow(reqId, i) as u64);
    //         i = i + 1;
    //     };
    //     assert!(amount > 0, EAMOUNT_CANNOT_BE_ZERO);
    //     amount
    // }

    // public(friend) fun amountFrom(reqId: &vector<u8>): u64 acquires ReqHelpersStorage {
    //     let storeR = borrow_global_mut<ReqHelpersStorage>(@free_tunnel_aptos);
    //     let amount = decodeAmount(reqId);
    //     let tokenIndex = decodeTokenIndex(reqId);
    //     let decimals = fungible_asset::decimals<Metadata>(*storeR.tokens.borrow(tokenIndex)) as u64;
    //     if (decimals > 6) {
    //         amount = amount * math64::pow(10, decimals - 6);
    //     } else if (decimals < 6) {
    //         amount = amount / math64::pow(10, 6 - decimals);
    //     };
    //     amount
    // }

    // public(friend) fun msgFromReqSigningMessage(reqId: &vector<u8>): vector<u8> {
    //     assert!(vector::length(reqId) == 32, EINVALID_REQ_ID_LENGTH);
    //     let specificAction = actionFrom(reqId) & 0x0f;
    //     if (specificAction == 1) {
    //         let msg = ETH_SIGN_HEADER();
    //         vector::append(&mut msg, smallU64ToString(3 + vector::length(&BRIDGE_CHANNEL()) + 29 + 66));
    //         vector::append(&mut msg, b"[");
    //         vector::append(&mut msg, BRIDGE_CHANNEL());
    //         vector::append(&mut msg, b"]\n");
    //         vector::append(&mut msg, b"Sign to execute a lock-mint:\n");
    //         vector::append(&mut msg, hexToString(reqId, true));
    //         msg
    //     } else if (specificAction == 2) {
    //         let msg = ETH_SIGN_HEADER();
    //         vector::append(&mut msg, smallU64ToString(3 + vector::length(&BRIDGE_CHANNEL()) + 31 + 66));
    //         vector::append(&mut msg, b"[");
    //         vector::append(&mut msg, BRIDGE_CHANNEL());
    //         vector::append(&mut msg, b"]\n");
    //         vector::append(&mut msg, b"Sign to execute a burn-unlock:\n");
    //         vector::append(&mut msg, hexToString(reqId, true));
    //         msg
    //     } else if (specificAction == 3) {
    //         let msg = ETH_SIGN_HEADER();
    //         vector::append(&mut msg, smallU64ToString(3 + vector::length(&BRIDGE_CHANNEL()) + 29 + 66));
    //         vector::append(&mut msg, b"[");
    //         vector::append(&mut msg, BRIDGE_CHANNEL());
    //         vector::append(&mut msg, b"]\n");
    //         vector::append(&mut msg, b"Sign to execute a burn-mint:\n");
    //         vector::append(&mut msg, hexToString(reqId, true));
    //         msg
    //     } else {
    //         vector::empty<u8>()
    //     }
    // }

    // public(friend) fun assertFromChainOnly(reqId: &vector<u8>) {
    //     assert!(CHAIN == *vector::borrow(reqId, 16), ENOT_FROM_CURRENT_CHAIN);
    // }

    // public(friend) fun assertToChainOnly(reqId: &vector<u8>) {
    //     assert!(CHAIN == *vector::borrow(reqId, 17), ENOT_TO_CURRENT_CHAIN);
    // }

    // #[test]
    // fun testDecodingReqid() {
}
