use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct Block {
    pub T: String,
    created: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    miner: Option<String>,
    nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    previd: Option<String>,
    txids: Vec<String>,
    #[serde(rename = "type")]
    block_type: String
}

impl Block {
    pub fn new(previd: Option<String>, miner: Option<String>, note: Option<String>, txids: Vec<String>) -> Self {
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Block {
            T: "00000000abc00000000000000000000000000000000000000000000000000000".to_string(),
            created: created,
            miner,
            nonce: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            note,
            previd,
            txids,
            block_type: "block".to_string(),
        }
    }

    pub fn canonical_json(&self) -> String {
        json_canon::to_string(self).unwrap()
    }

    pub fn blockid(&self) -> [u8; 32] {
        crate::blake2s::blake2s(self.canonical_json().as_bytes())
    }

    pub fn from_canonical_json(json: &str) -> Block {
        serde_json::from_str(json).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let genesis = Block {
            T: "00000000abc00000000000000000000000000000000000000000000000000000".to_string(),
            created: 1771159355,
            miner: Some("Marabu".to_string()),
            nonce: "00dd82159556175752d9ba7349df67bddd237b59183747383f7b720e85c32347".to_string(),
            note: Some("Financial Times 2026-02-13: Crypto's battle with the banks is splitting Trump's base".to_string()),
            previd: None,
            txids: vec!(),
            block_type: "block".to_string()
        };
        let genesis_blockid = "00000000522473196b73bc619a8b18472c4cb4c6caf785a13fa32aaae7222ff6";
        assert_eq!(hex::encode(genesis.blockid()), genesis_blockid)
    }
}
