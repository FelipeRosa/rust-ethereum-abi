# Ethereum ABI

[![Crates.io](https://img.shields.io/crates/v/ethereum_abi)](https://crates.io/crates/ethereum_abi)
[![Docs.rs](https://docs.rs/ethereum_abi/badge.svg)](https://docs.rs/ethereum_abi)

_Currently under development._

`ethereum_abi` is a Rust library to help writing code that interacts with Ethereum Smart Contracts.

## Example

```rust
use std::fs::File;
use std::io;

use ethereum_abi::Abi;

fn main() {
    // Parse ABI JSON file
    let abi = {
        let file = File::open("some_abi.json").expect("failed to open ABI file");

        Abi::from_reader(file).expect("failed to parse ABI")
    };

    // Read some ABI encoded function input
    let mut encoded_input = String::new();
    io::stdin()
        .read_line(&mut encoded_input)
        .expect("failed to read encoded input");

    // Decode
    let (func, decoded_input) = abi
        .decode_input_from_hex(&encoded_input.trim())
        .expect("failed decoding input");

    println!(
        "function called: {}\ninput: {:?}",
        func.name, decoded_input.index_params
    );
}
```

## Features

### ABI encoder V1

- [x] JSON parsing
- [x] Function selectors (method ID)
- [x] argument encoding and decoding

### ABI encoder V2

- [x] JSON parsing
- [x] Function selectors (method ID)
- [x] argument encoding and decoding

## License

This project is licensed under the [MIT License]

[MIT License]: https://github.com/FelipeRosa/rust-ethereum-abi/blob/main/LICENSE
