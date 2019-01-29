use super::*;

#[test]
fn get_subwork() {
    let work = r#"[
        "0",
        "53295d842611768501295be6a3305f7cc28a70e00016c0380000000000000000",
        "02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4b03d08d08042d1c505c612f4254432e434f4d2ffabe6d6d54bf3732a3dc252297cf75d4c1cf35878ed99626ef2ffb311cef0cc7c4eff06e0100000000000000",
        "ffffffff0329e74d4c0000000016001497cfc76442fe717f2a3f0cc9c175f7561b6619970000000000000000266a24aa21a9ed82b1c33e59cfca82f3af6b51d6094775df97a385f135cabb259ca9fdb63f124b00000000000000002952534b424c4f434b3af5fbe7f0043226e246965f4e7db2c3ff6d5dfedb9b85d0873eed8cca4227c14900000000",
        [
            "0c3c1a888c2b9e521c3c1456414473b712216568c3a69e7eefe6434134f951ed",
            "f12de771dc657d5e24c0737c444ab284222997bf3e9c9d298e72effa8cbcde5a",
            "1236c0d90296a8be77d6fa1592da80e1b7cc4786dc2c0450bda264585d77c54f",
            "59ca1f1c9dcdc854391af53899c261f0e2ffab2dc8378a18b1a9814d08d1e19c",
            "4a35546b633381f873d5e45c00c538cbfd315d9caab314b4d6f1c61ee139740f",
            "f905d59a405db965a9fa459a7f0b3e8f76eaf8f9b97d71d4a10aa6008bf71e74",
            "623276bd848189e535ede317e7736b14f3582ddb5134dbfbf7e6f86fc627e7fe",
            "f9207de81abf714acf8b995e34c3e1143ca23464f1003f18896904f44f3732eb",
            "4544bb603bd0b635e57dd3514b4c68f1ae232b170213194fc3a8cce087ba4387",
            "c60399ab19b9a8bf5600c84419ecaee2f830031164255fe4549f90389239acc8",
            "98facdad2e8e9747cc3eccd240112e0e235201083c3f09dcc1989103b7e4640f",
            "ae2817984d528bf9174f10ecef74dc673086fbfaa5b61f3f8fdf12b9309a8a34"
        ],
        "20000000",
        "17306835",
        "5c501c2a",
        true
    ]"#;

    let work: Work = serde_json::from_str(work).unwrap();
    let xnonce = (
        &Bytes::from("72e03131".from_hex().unwrap()),
        Bytes::from("0000000000000001".from_hex().unwrap()),
    );

    let block_header = Bytes::from("2000000053295d842611768501295be6a3305f7cc28a70e00016c038000000000000000009a2beeef9c314bfe0c9f839b80bb8724247e630aa6f1efed1e6a483cd1cc8e85c501c2a17306835".from_hex().unwrap());
    let midstate = Bytes::from(
        "de968ef4901a148bc3128e87c1db85c152def4772f5e256b98c4ba060e0707c5"
            .from_hex()
            .unwrap(),
    );

    let subwork = work.subwork(xnonce.clone());
    assert_eq!(block_header, &subwork.block_header);
    assert_eq!(midstate, &subwork.midstate);

    //    let chunk1_itor = Chunk1Itor::new(&work, &xnonce, 0x1fffe000u32);
    //    for chunk1 in chunk1_itor {
    //        println!("{:?}", chunk1);
    //    }
}
