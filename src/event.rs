use anyhow::{anyhow, Result};
use ethereum_types::H256;
use std::collections::VecDeque;

use crate::{DecodedParams, Param, Type, Value};

/// Contract Error Definition
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    /// Error name.
    pub name: String,
    /// Error inputs.
    pub inputs: Vec<Param>,
}

/// Contract event definition.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Event {
    /// Event name.
    pub name: String,
    /// Event inputs.
    pub inputs: Vec<Param>,
    /// Whether the event is anonymous or not.
    pub anonymous: bool,
}

impl Event {
    /// Returns the event's signature.
    pub fn signature(&self) -> String {
        format!(
            "{}({})",
            self.name,
            self.inputs
                .iter()
                .map(|param| param.type_.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Compute the event's topic hash
    pub fn topic(&self) -> H256 {
        use tiny_keccak::{Hasher, Keccak};

        let mut keccak_out = [0u8; 32];
        let mut hasher = Keccak::v256();
        hasher.update(self.signature().as_bytes());
        hasher.finalize(&mut keccak_out);

        H256::from_slice(&keccak_out)
    }

    /// Decode event params from a log's topics and data.
    pub fn decode_data_from_slice(
        &self,
        mut topics: &[H256],
        data: &[u8],
    ) -> Result<DecodedParams> {
        // strip event topic from the topics array
        // so that we end up with only the values we
        // need to decode
        if !self.anonymous {
            topics = topics
                .get(1..)
                .ok_or_else(|| anyhow!("missing event topic"))?;
        }

        let mut topics_values = VecDeque::from(topics.to_vec());

        let mut data_values = VecDeque::from(Value::decode_from_slice(
            data,
            &self
                .inputs
                .iter()
                .filter(|input| !input.indexed.unwrap_or(false))
                .map(|input| input.type_.clone())
                .collect::<Vec<_>>(),
        )?);

        let mut decoded = vec![];
        for input in self.inputs.iter().cloned() {
            let decoded_value = if input.indexed.unwrap_or(false) {
                let val = topics_values
                    .pop_front()
                    .ok_or_else(|| anyhow!("insufficient topics entries"))?;

                let bytes = val.to_fixed_bytes().to_vec();

                if Self::is_encoded_to_keccak(&input.type_) {
                    Ok(Value::FixedBytes(bytes))
                } else {
                    Value::decode_from_slice(&bytes, &[input.type_.clone()])?
                        .first()
                        .ok_or_else(|| anyhow!("no value decoded from topics entry"))
                        .map(Clone::clone)
                }
            } else {
                data_values
                    .pop_front()
                    .ok_or_else(|| anyhow!("insufficient data values"))
            };

            decoded.push((input, decoded_value?));
        }

        Ok(DecodedParams::from(decoded))
    }

    fn is_encoded_to_keccak(ty: &Type) -> bool {
        matches!(
            ty,
            Type::FixedArray(_, _) | Type::Array(_) | Type::Bytes | Type::String | Type::Tuple(_)
        )
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{Abi, DecodedParams, Type};

    use super::*;

    use ethereum_types::U256;
    use pretty_assertions::assert_eq;

    fn test_event() -> Event {
        Event {
            name: "Approve".to_string(),
            inputs: vec![
                Param {
                    name: "x".to_string(),
                    type_: Type::Uint(56),
                    indexed: Some(true),
                },
                Param {
                    name: "y".to_string(),
                    type_: Type::String,
                    indexed: Some(true),
                },
            ],
            anonymous: false,
        }
    }

    #[test]
    fn test_signature() {
        let evt = test_event();

        assert_eq!(evt.signature(), "Approve(uint56,string)");
    }

    #[test]
    fn test_topic() {
        let evt = test_event();

        assert_eq!(
            evt.topic(),
            H256::from_str("a61d695a23b25aa2db668e3216af77ef9a2409384ddff9e6a94bfd50a32c6eeb")
                .unwrap()
        );
    }

    #[test]
    fn test_decode_data_from_slice() {
        let topics: Vec<_> = [
            "f5108f9bff51ebdc9f23cf7c976feee4dbda0ac72bb6120bf0256adc72a28e68",
            "000000000000000000000000000000000000000000000000000000000000000a",
            "000000000000000000000000000000000000000000000000000000000000000b",
        ]
        .iter()
        .map(|h| H256::from_str(h).unwrap())
        .collect();

        let data = hex::decode("00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000036162630000000000000000000000000000000000000000000000000000000000").unwrap();

        let x = Param {
            name: "x".to_string(),
            type_: Type::Uint(256),
            indexed: None,
        };
        let y = Param {
            name: "y".to_string(),
            type_: Type::Uint(256),
            indexed: Some(true),
        };
        let x1 = Param {
            name: "x1".to_string(),
            type_: Type::Uint(256),
            indexed: None,
        };
        let y1 = Param {
            name: "y1".to_string(),
            type_: Type::Uint(256),
            indexed: Some(true),
        };
        let s = Param {
            name: "s".to_string(),
            type_: Type::String,
            indexed: None,
        };

        let evt = Event {
            name: "Test".to_string(),
            inputs: vec![x.clone(), y.clone(), x1.clone(), y1.clone(), s.clone()],
            anonymous: false,
        };

        let abi = Abi {
            constructor: None,
            functions: vec![],
            events: vec![evt],
            errors: vec![],
            has_receive: false,
            has_fallback: false,
        };

        assert_eq!(
            abi.decode_log_from_slice(&topics, &data)
                .expect("decode_log_from_slice failed"),
            (
                &abi.events[0],
                DecodedParams::from(vec![
                    (x, Value::Uint(U256::from(1), 256)),
                    (y, Value::Uint(U256::from(10), 256)),
                    (x1, Value::Uint(U256::from(2), 256)),
                    (y1, Value::Uint(U256::from(11), 256)),
                    (s, Value::String("abc".to_string()))
                ])
            )
        );
    }
}
