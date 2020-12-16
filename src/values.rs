use ethereum_types::{H160, U256};

use crate::types::Type;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Uint(U256, usize),
    Int(U256, usize),
    Address(H160),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Tuple(Vec<Value>),
}

impl Value {
    pub fn decode_from_slice(bs: &[u8], tys: &Vec<Type>) -> Result<Vec<Value>, String> {
        tys.iter()
            .try_fold((vec![], 0), |(mut values, at), ty| {
                let (value, consumed) = Self::decode(bs, ty, 0, at)?;
                values.push(value);

                Ok((values, at + consumed))
            })
            .map(|(values, _)| values)
    }

    fn decode(bs: &[u8], ty: &Type, base_addr: usize, at: usize) -> Result<(Value, usize), String> {
        let dec = match ty {
            Type::Uint(size) => {
                let at = base_addr + at;
                let uint = U256::from_big_endian(&bs[at..(at + 32)]);

                Ok((Value::Uint(uint, *size), 32))
            }

            Type::Int(size) => {
                let at = base_addr + at;
                let uint = U256::from_big_endian(&bs[at..(at + 32)]);

                Ok((Value::Int(uint, *size), 32))
            }

            Type::Address => {
                let at = base_addr + at;
                let addr = H160::from_slice(&bs[at..(at + 20)]);

                Ok((Value::Address(addr), 32))
            }

            Type::Bool => {
                let at = base_addr + at;
                let b = U256::from_big_endian(&bs[at..(at + 32)]) == U256::one();

                Ok((Value::Bool(b), 32))
            }

            Type::FixedBytes(size) => {
                let at = base_addr + at;
                let bv = bs[at..(at + size)].to_vec();

                Ok((Value::Bytes(bv), Self::padded32_size(*size)))
            }

            Type::FixedArray(ty, size) => {
                let (base_addr, at) = if ty.is_dynamic() {
                    // For fixed arrays of types that are dynamic, we just jump
                    // to the offset location and decode from there.
                    let offset = U256::from_big_endian(&bs[at..(at + 32)]).as_usize();

                    (base_addr + offset, 0)
                } else {
                    // There's no need to change the addressing because fixed arrays
                    // will consume input by calling decode recursively and addressing
                    // will be computed correctly inside those calls.
                    (base_addr, at)
                };

                (0..(*size))
                    .try_fold((vec![], 0), |(mut values, total_consumed), _| {
                        let (value, consumed) =
                            Self::decode(bs, ty, base_addr, at + total_consumed)?;

                        values.push(value);

                        Ok((values, total_consumed + consumed))
                    })
                    .map(|(values, consumed)| {
                        let consumed = if ty.is_dynamic() { 32 } else { consumed };

                        (Value::Array(values), consumed)
                    })
            }

            Type::String => {
                let at = base_addr + at;
                let (bytes_value, consumed) = Self::decode(bs, &Type::Bytes, base_addr, at)?;

                let bytes = if let Value::Bytes(bytes) = bytes_value {
                    bytes
                } else {
                    // should always be Value::Bytes
                    unreachable!();
                };

                let s = String::from_utf8(bytes).map_err(|e| e.to_string())?;

                Ok((Value::String(s), consumed))
            }

            Type::Bytes => {
                let at = base_addr + at;
                let offset = U256::from_big_endian(&bs[at..(at + 32)]).as_usize();

                let at = base_addr + offset;
                let bytes_len = U256::from_big_endian(&bs[at..(at + 32)]).as_usize();

                let at = at + 32;
                let bytes = bs[at..(at + bytes_len)].to_vec();

                // consumes only the first 32 bytes, i.e. the offset pointer
                Ok((Value::Bytes(bytes), 32))
            }

            Type::Array(ty) => {
                let at = base_addr + at;
                let offset = U256::from_big_endian(&bs[at..(at + 32)]).as_usize();

                let at = base_addr + offset;
                let array_len = U256::from_big_endian(&bs[at..(at + 32)]).as_usize();

                let (arr, _) = Self::decode(bs, &Type::FixedArray(ty.clone(), array_len), at, 32)?;

                Ok((arr, 32))
            }

            Type::Tuple(_) => todo!(),
        };

        dec
    }

    // Computes the padded size for a given size, e.g.:
    // padded32_size(20) == 32
    // padded32_size(32) == 32
    // padded32_size(40) == 64
    fn padded32_size(size: usize) -> usize {
        let r = size % 32;

        if r == 0 {
            size
        } else {
            size + 32 - r
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rand::Rng;

    #[test]
    fn decode_uint() {
        let uint: U256 = U256::exp10(18) + 1;

        let mut bs = [0u8; 32];
        uint.to_big_endian(&mut bs[..]);

        let v = Value::decode_from_slice(&bs, &vec![Type::Uint(256)]);

        assert_eq!(v, Ok(vec![Value::Uint(uint, 256)]));
    }

    #[test]
    fn decode_int() {
        let uint: U256 = U256::exp10(18) + 1;

        let mut bs = [0u8; 32];
        uint.to_big_endian(&mut bs[..]);

        let v = Value::decode_from_slice(&bs, &vec![Type::Int(256)]);

        assert_eq!(v, Ok(vec![Value::Int(uint, 256)]));
    }

    #[test]
    fn decode_address() {
        let addr = H160::random();

        let mut bs = [0u8; 32];
        &bs[0..20].copy_from_slice(addr.as_bytes());

        let v = Value::decode_from_slice(&bs, &vec![Type::Address]);

        assert_eq!(v, Ok(vec![Value::Address(addr)]));
    }

    #[test]
    fn decode_bool() {
        let mut bs = [0u8; 32];
        bs[31] = 1;

        let v = Value::decode_from_slice(&bs, &vec![Type::Bool]);

        assert_eq!(v, Ok(vec![Value::Bool(true)]));
    }

    #[test]
    fn decode_fixed_bytes() {
        let mut bs = [0u8; 32];
        for i in 1..16 {
            bs[i] = i as u8;
        }

        let v = Value::decode_from_slice(&bs, &vec![Type::FixedBytes(16)]);

        assert_eq!(v, Ok(vec![Value::Bytes(bs[0..16].to_vec())]));
    }

    #[test]
    fn decode_fixed_array() {
        let mut bs = [0u8; 128];

        // encode some data
        let uint1 = U256::from(5);
        let uint2 = U256::from(6);
        let uint3 = U256::from(7);
        let uint4 = U256::from(8);

        uint1.to_big_endian(&mut bs[0..32]);
        uint2.to_big_endian(&mut bs[32..64]);
        uint3.to_big_endian(&mut bs[64..96]);
        uint4.to_big_endian(&mut bs[96..128]);

        let uint_arr2 = Type::FixedArray(Box::new(Type::Uint(256)), 2);

        let v = Value::decode_from_slice(&bs, &vec![Type::FixedArray(Box::new(uint_arr2), 2)]);

        assert_eq!(
            v,
            Ok(vec![Value::Array(vec![
                Value::Array(vec![Value::Uint(uint1, 256), Value::Uint(uint2, 256)]),
                Value::Array(vec![Value::Uint(uint3, 256), Value::Uint(uint4, 256)])
            ])])
        );
    }

    #[test]
    fn decode_string() {
        let mut rng = rand::thread_rng();

        let mut bs = [0u8; 128];

        bs[31] = 0x20; // big-endian string offset

        let str_len: usize = rng.gen_range(0, 64);
        bs[63] = str_len as u8; // big-endian string size

        let chars = "abcdef0123456789".as_bytes();

        for i in 0..(str_len as usize) {
            bs[64 + i] = chars[rng.gen_range(0, chars.len())];
        }

        let v = Value::decode_from_slice(&bs, &vec![Type::String]);

        let expected_str = String::from_utf8(bs[64..(64 + str_len)].to_vec()).unwrap();
        assert_eq!(v, Ok(vec![Value::String(expected_str)]));
    }

    #[test]
    fn decode_bytes() {
        let mut rng = rand::thread_rng();

        let mut bs = [0u8; 128];
        bs[31] = 0x20; // big-endian bytes offset

        let bytes_len: usize = rng.gen_range(0, 64);
        bs[63] = bytes_len as u8; // big-endian bytes length

        for i in 0..(bytes_len as usize) {
            bs[64 + i] = rng.gen();
        }

        let v = Value::decode_from_slice(&bs, &vec![Type::Bytes]);

        assert_eq!(v, Ok(vec![Value::Bytes(bs[64..(64 + bytes_len)].to_vec())]));
    }

    #[test]
    fn decode_array() {
        let mut bs = [0u8; 192];
        bs[31] = 0x20; // big-endian array offset
        bs[63] = 2; // big-endian array length

        // encode some data
        let uint1 = U256::from(5);
        let uint2 = U256::from(6);
        let uint3 = U256::from(7);
        let uint4 = U256::from(8);

        uint1.to_big_endian(&mut bs[64..96]);
        uint2.to_big_endian(&mut bs[96..128]);
        uint3.to_big_endian(&mut bs[128..160]);
        uint4.to_big_endian(&mut bs[160..192]);

        let uint_arr2 = Type::FixedArray(Box::new(Type::Uint(256)), 2);

        let v = Value::decode_from_slice(&bs, &vec![Type::Array(Box::new(uint_arr2))]);

        assert_eq!(
            v,
            Ok(vec![Value::Array(vec![
                Value::Array(vec![Value::Uint(uint1, 256), Value::Uint(uint2, 256)]),
                Value::Array(vec![Value::Uint(uint3, 256), Value::Uint(uint4, 256)])
            ])])
        );
    }

    #[test]
    fn decode_many() {
        // function f(string memory x, uint32 y, uint32[][2] memory z)
        let tys = vec![
            Type::String,
            Type::Uint(32),
            Type::FixedArray(Box::new(Type::Array(Box::new(Type::Uint(32)))), 2),
        ];

        // f("abc", 5, [[1, 2], [3]])
        let input = "0000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000036162630000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000003";
        let mut bs = [0u8; 384];
        hex::decode_to_slice(input, &mut bs).unwrap();

        let v = Value::decode_from_slice(&bs, &tys);
        println!("{:?}", v);

        assert_eq!(
            v,
            Ok(vec![
                Value::String("abc".to_string()),
                Value::Uint(U256::from(5), 32),
                Value::Array(vec![
                    Value::Array(vec![
                        Value::Uint(U256::from(1), 32),
                        Value::Uint(U256::from(2), 32),
                    ]),
                    Value::Array(vec![Value::Uint(U256::from(3), 32)]),
                ]),
            ]),
        );
    }
}
