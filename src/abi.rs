use serde::{de::Visitor, Deserialize};

use crate::params::Param;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Abi {
    pub constructor: Option<Constructor>,
    pub functions: Vec<Function>,
    pub events: Vec<Event>,
    pub has_receive: bool,
    pub has_fallback: bool,
}

impl Abi {
    pub fn from_str(s: &str) -> Result<Abi, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    pub fn from_reader<R>(rdr: R) -> Result<Abi, String>
    where
        R: std::io::Read,
    {
        serde_json::from_reader(rdr).map_err(|e| e.to_string())
    }
}

impl<'de> Deserialize<'de> for Abi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(AbiVisitor)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbiEntry {
    #[serde(rename = "type")]
    type_: String,
    name: Option<String>,
    inputs: Option<Vec<Param>>,
    outputs: Option<Vec<Param>>,
    state_mutability: Option<StateMutability>,
    anonymous: Option<bool>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Constructor {
    pub inputs: Vec<Param>,
    pub state_mutability: StateMutability,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Function {
    pub name: String,
    pub inputs: Vec<Param>,
    pub outputs: Vec<Param>,
    pub state_mutability: StateMutability,
}

impl Function {
    pub fn method_id(&self) -> [u8; 4] {
        use tiny_keccak::{Hasher, Keccak};

        let mut keccak_out = [0u8; 32];
        let mut hasher = Keccak::v256();
        hasher.update(self.signature().as_bytes());
        hasher.finalize(&mut keccak_out);

        let mut mid = [0u8; 4];
        mid.copy_from_slice(&keccak_out[0..4]);

        mid
    }

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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Event {
    pub name: String,
    pub inputs: Vec<Param>,
    pub anonymous: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateMutability {
    Payable,
    NonPayable,
    View,
    Pure,
}

struct AbiVisitor;

impl<'de> Visitor<'de> for AbiVisitor {
    type Value = Abi;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "ABI")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut abi = Abi {
            constructor: None,
            functions: vec![],
            events: vec![],
            has_receive: false,
            has_fallback: false,
        };

        while let Ok(Some(entry)) = seq.next_element::<AbiEntry>() {
            match entry.type_.as_str() {
                "receive" => abi.has_receive = true,

                "fallback" => abi.has_fallback = true,

                "constructor" => {
                    let state_mutability = entry.state_mutability.ok_or_else(|| {
                        serde::de::Error::custom("missing constructor state mutability".to_string())
                    })?;

                    let inputs = entry.inputs.unwrap_or_default();

                    abi.constructor = Some(Constructor {
                        inputs,
                        state_mutability,
                    });
                }

                "function" => {
                    let state_mutability = entry.state_mutability.ok_or_else(|| {
                        serde::de::Error::custom("missing constructor state mutability".to_string())
                    })?;

                    let inputs = entry.inputs.unwrap_or_default();

                    let outputs = entry.outputs.unwrap_or_default();

                    let name = entry.name.ok_or_else(|| {
                        serde::de::Error::custom("missing function name".to_string())
                    })?;

                    abi.functions.push(Function {
                        name,
                        inputs,
                        outputs,
                        state_mutability,
                    });
                }

                "event" => {
                    let inputs = entry.inputs.unwrap_or_default();

                    let name = entry.name.ok_or_else(|| {
                        serde::de::Error::custom("missing function name".to_string())
                    })?;

                    let anonymous = entry.anonymous.ok_or_else(|| {
                        serde::de::Error::custom("missing event anonymous field".to_string())
                    })?;

                    abi.events.push(Event {
                        name,
                        inputs,
                        anonymous,
                    });
                }

                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "invalid ABI entry type: {}",
                        entry.type_
                    )))
                }
            }
        }

        Ok(abi)
    }
}

#[cfg(test)]
mod test {
    use crate::types::Type;

    use super::*;

    fn test_function() -> Function {
        Function {
            name: "funname".to_string(),
            inputs: vec![
                Param {
                    name: "".to_string(),
                    type_: Type::Address,
                    indexed: None,
                },
                Param {
                    name: "x".to_string(),
                    type_: Type::FixedArray(Box::new(Type::Uint(56)), 5),
                    indexed: None,
                },
            ],
            outputs: vec![],
            state_mutability: StateMutability::Pure,
        }
    }

    #[test]
    fn function_signature() {
        let fun = test_function();
        assert_eq!(fun.signature(), "funname(address,uint56[5])");
    }

    #[test]
    fn function_method_id() {
        let fun = test_function();
        assert_eq!(fun.method_id(), [0xab, 0xa0, 0xe6, 0x3a]);
    }

    #[test]
    fn works() {
        let s = r#"[{"inputs":[{"internalType":"address","name":"a","type":"address"}],"stateMutability":"nonpayable","type":"constructor"},{"anonymous":false,"inputs":[{"indexed":false,"internalType":"address","name":"x","type":"address"},{"indexed":false,"internalType":"uint256","name":"y","type":"uint256"}],"name":"E","type":"event"},{"inputs":[{"internalType":"uint256","name":"x","type":"uint256"}],"name":"f","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"nonpayable","type":"function"},{"stateMutability":"payable","type":"receive"}]"#;
        let abi = Abi::from_str(s).unwrap();

        assert_eq!(
            abi,
            Abi {
                constructor: Some(Constructor {
                    inputs: vec![Param {
                        name: "a".to_string(),
                        type_: Type::Address,
                        indexed: None
                    }],
                    state_mutability: StateMutability::NonPayable
                }),
                functions: vec![Function {
                    name: "f".to_string(),
                    inputs: vec![Param {
                        name: "x".to_string(),
                        type_: Type::Uint(256),
                        indexed: None
                    }],
                    outputs: vec![Param {
                        name: "".to_string(),
                        type_: Type::Uint(256),
                        indexed: None
                    }],
                    state_mutability: StateMutability::NonPayable
                }],
                events: vec![Event {
                    name: "E".to_string(),
                    inputs: vec![
                        Param {
                            name: "x".to_string(),
                            type_: Type::Address,
                            indexed: Some(false)
                        },
                        Param {
                            name: "y".to_string(),
                            type_: Type::Uint(256),
                            indexed: Some(false)
                        }
                    ],
                    anonymous: false
                }],
                has_receive: true,
                has_fallback: false
            }
        )
    }
}
