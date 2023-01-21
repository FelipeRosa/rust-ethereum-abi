use std::fs::File;

use ethereum_abi::Abi;

fn main() {
    // Parse ABI from file
    let abi: Abi = {
        let file =
            File::open("examples/uniswapv3factory_abi.json").expect("failed to open ABI file");

        serde_json::from_reader(file).expect("failed to parse ABI")
    };

    // Decode contract input
    // From: https://etherscan.io/tx/0x535e880ab0d966fbc7a354c322046fe6f01581e94b0d9b76a12683feefb98481
    let encoded_input = "a1671295000000000000000000000000a0b211418d87c9f5918e6213fec3b13290aa5f26000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000000000000000000000000000000000000000bb8";
    let (func, decoded_input) = abi
        .decode_input_from_hex(&encoded_input.trim())
        .expect("failed decoding input");

    println!("function called: {}\ninput: {:?}", func.name, decoded_input);
}
