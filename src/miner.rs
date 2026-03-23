use crate::{blake2s::blake2s, block::Block};
use rand::Rng;

fn increment_nonce(nonce: &mut [u8; 32]) {
    for i in (0..32).rev() {
        if nonce[i] < 255 {
            nonce[i] += 1;
            break;
        }
        nonce[i] = 0;
    }
}

fn inject_nonce(json: &mut String, nonce: &[u8; 32]) {
    let prefix = "\"nonce\":\"";
    let prefix_index = json.find(prefix).unwrap();
    let nonce_start = prefix_index + prefix.len();
    let nonce_bytes = unsafe { json[nonce_start..nonce_start + 64].as_bytes_mut() };
    let hex_nonce = hex::encode(nonce);
    nonce_bytes.copy_from_slice(hex_nonce.as_bytes());
}

pub fn mine(block: Block) -> (Block, u64) {
    let mut block_json = block.canonical_json();
    let mut nonce = [0_u8; 32];
    rand::thread_rng().fill(&mut nonce);
    inject_nonce(&mut block_json, &nonce);

    let target: [u8; 32] = hex::decode(&block.T).unwrap().try_into().unwrap();
    let mut hashes: u64 = 0;
    loop {
        hashes += 1;
        if blake2s(block_json.as_bytes()) < target {
            break;
        }

        increment_nonce(&mut nonce);
        inject_nonce(&mut block_json, &nonce);
    }

    (Block::from_canonical_json(&block_json), hashes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mining() {
        let mut block = Block::new(None, Some("test".to_string()), None, vec![]);
        block.T = "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
        let (mined, _) = mine(block);
        let blockid = mined.blockid();
        let target = hex::decode(mined.T).unwrap().try_into().unwrap();
        assert!(blockid < target);
    }
}
