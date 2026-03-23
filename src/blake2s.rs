const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

const ROTATIONS: [u32; 4] = [16, 12, 8, 7];

const W: u32 = 32;
const R: usize = 10;

fn g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, x: u32, y: u32) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(ROTATIONS[0]);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(ROTATIONS[1]);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(ROTATIONS[2]);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(ROTATIONS[3]);
}

fn compress(h: &mut [u32; 8], m: &[u32; 16], t: u64, f: bool) {
    let mut v: [u32; 16] = [0_u32; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV);

    v[12] ^= t as u32;
    v[13] ^= (t >> W) as u32;

    if f {
        v[14] ^= !0_u32;
    }

    for i in 0..R {
        let mut s: [usize; 16] = [0_usize; 16];
        s.copy_from_slice(&SIGMA[i % 10]);
        g(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        g(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        g(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        g(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);

        g(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        g(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        g(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        g(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    for i in 0..8 {
        h[i] = h[i] ^ v[i] ^ v[i + 8]
    }
}

pub fn blake2s(input: &[u8]) -> [u8; 32] {
    let mut h = IV;
    h[0] ^= 0x01010020;
    let mut output = [0u8; 32];

    if input.is_empty() {
        let m = [0_u32; 16];
        compress(&mut h, &m, 0, true);

        for i in 0..8 {
            let bytes = h[i].to_le_bytes();
            output[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
        return output;
    }

    let mut chunks = input.chunks(64).peekable();
    let mut t: u64 = 0;

    while let Some(chunk) = chunks.next() {
        let mut block = [0_u8; 64];
        block[..chunk.len()].copy_from_slice(chunk);
        t += chunk.len() as u64;
        let is_last = chunks.peek().is_none();

        let mut m = [0_u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        compress(&mut h, &m, t, is_last);
    }

    for i in 0..8 {
        let bytes = h[i].to_le_bytes();
        output[i * 4..i * 4 + 4].copy_from_slice(&bytes);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = blake2s(b"");
        let expected =
            hex::decode("69217a3079908094e11121d042354a7c1f55b6482ca1a51e1b250dfd1ed0eef9")
                .unwrap();
        assert_eq!(result, expected.as_slice());
    }

    #[test]
    fn test_abc() {
        let result = blake2s(b"abc");
        let expected =
            hex::decode("508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982")
                .unwrap();
        assert_eq!(result, expected.as_slice());
    }

    #[test]
    fn test_against_reference() {
        use blake2::{Blake2s256, Digest};
        let input = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let reference = Blake2s256::digest(input);
        let ours = blake2s(input);
        assert_eq!(ours, reference.as_slice());
    }

    #[test]
    fn test_genesis() {
        use blake2::{Blake2s256, Digest};
        let input = b"{\"T\":\"00000000abc00000000000000000000000000000000000000000000000000000\",\"created\":1771159355,\"miner\":\"Marabu\",\"nonce\":\"00dd82159556175752d9ba7349df67bddd237b59183747383f7b720e85c32347\",\"note\":\"Financial Times 2026-02-13: Crypto's battle with the banks is splitting Trump's base\",\"previd\":null,\"txids\":[],\"type\":\"block\"}";
        let reference = Blake2s256::digest(input);
        let ours = blake2s(input);
        let expected =
            hex::decode("00000000522473196b73bc619a8b18472c4cb4c6caf785a13fa32aaae7222ff6")
                .unwrap();
        assert_eq!(ours, reference.as_slice());
        assert_eq!(ours, expected.as_slice());
    }
}
