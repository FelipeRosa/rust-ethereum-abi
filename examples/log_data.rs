use std::{fs::File, str::FromStr};

use ethereum_abi::Abi;
use ethereum_types::H256;

fn main() {
    // Parse ABI JSON file
    let abi: Abi = {
        let file =
            File::open("examples/uniswapv3factory_abi.json").expect("failed to open ABI file");

        serde_json::from_reader(file).expect("failed to parse ABI")
    };

    // Log data
    // Taken from: https://etherscan.io/tx/0x535e880ab0d966fbc7a354c322046fe6f01581e94b0d9b76a12683feefb98481#eventlog
    let topics = vec![
        H256::from_str("783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118").unwrap(),
        H256::from_str("000000000000000000000000a0b211418d87c9f5918e6213fec3b13290aa5f26").unwrap(),
        H256::from_str("000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap(),
        H256::from_str("0000000000000000000000000000000000000000000000000000000000000bb8").unwrap(),
    ];

    let data = "000000000000000000000000000000000000000000000000000000000000003c000000000000000000000000acabbea9c2d0ff835418d139d6a570b5025be085".as_bytes();

    // Decode
    let (evt, decoded_data) = abi
        .decode_log_from_slice(&topics, data)
        .expect("failed decoding log");

    println!("event: {}\ndata: {:?}", evt.name, decoded_data);
}
