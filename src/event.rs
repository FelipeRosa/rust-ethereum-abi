use ethereum_types::H256;

use crate::Param;

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
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::Type;

    use super::*;

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
}
