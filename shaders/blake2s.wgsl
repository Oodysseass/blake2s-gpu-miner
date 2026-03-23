@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read> difficulty: array<u32, 8>;
@group(0) @binding(2) var<storage, read_write> result: array<u32, 9>;
struct Params {
    input_len: u32,
    nonce_byte_offset: u32,
    batch_offset: u32
}
@group(0) @binding(3) var<uniform> params: Params;

const IV: array<u32, 8> = array<u32, 8>(
    0x6A09E667u, 0xBB67AE85u, 0x3C6EF372u, 0xA54FF53Au,
    0x510E527Fu, 0x9B05688Cu, 0x1F83D9ABu, 0x5BE0CD19u,
);

const SIGMA: array<array<u32, 16>, 10> = array<array<u32, 16>, 10>(
    array<u32, 16>(0u,1u,2u,3u,4u,5u,6u,7u,8u,9u,10u,11u,12u,13u,14u,15u),
    array<u32, 16>(14u,10u,4u,8u,9u,15u,13u,6u,1u,12u,0u,2u,11u,7u,5u,3u),
    array<u32, 16>(11u,8u,12u,0u,5u,2u,15u,13u,10u,14u,3u,6u,7u,1u,9u,4u),
    array<u32, 16>(7u,9u,3u,1u,13u,12u,11u,14u,2u,6u,5u,10u,4u,0u,15u,8u),
    array<u32, 16>(9u,0u,5u,7u,2u,4u,10u,15u,14u,1u,11u,12u,6u,8u,3u,13u),
    array<u32, 16>(2u,12u,6u,10u,0u,11u,8u,3u,4u,13u,7u,5u,15u,14u,1u,9u),
    array<u32, 16>(12u,5u,1u,15u,14u,13u,4u,10u,0u,7u,6u,3u,9u,2u,8u,11u),
    array<u32, 16>(13u,11u,7u,14u,12u,1u,3u,9u,5u,0u,15u,4u,8u,6u,2u,10u),
    array<u32, 16>(6u,15u,14u,9u,11u,3u,0u,8u,12u,2u,13u,7u,1u,4u,10u,5u),
    array<u32, 16>(10u,2u,8u,4u,7u,6u,1u,5u,15u,11u,9u,14u,3u,12u,13u,0u),
);

const ROTATIONS: array<u32, 4> = array<u32, 4>(
    16u, 12u, 8u, 7u
);

const W: u32 = 32u;
const R: u32 = 10u;

fn rotate_right(x: u32, n: u32) -> u32 {
    return (x >> n) | (x << (32u - n));
}

fn g(v: ptr<function, array<u32, 16>>, a: u32, b: u32, c: u32, d: u32, x: u32, y: u32) {
    (*v)[a] = (*v)[a] + (*v)[b] + x;
    (*v)[d] = rotate_right((*v)[d] ^ (*v)[a], ROTATIONS[0]);
    (*v)[c] = (*v)[c] + (*v)[d];
    (*v)[b] = rotate_right((*v)[b] ^ (*v)[c], ROTATIONS[1]);
    (*v)[a] = (*v)[a] + (*v)[b] + y;
    (*v)[d] = rotate_right((*v)[d] ^ (*v)[a], ROTATIONS[2]);
    (*v)[c] = (*v)[c] + (*v)[d];
    (*v)[b] = rotate_right((*v)[b] ^ (*v)[c], ROTATIONS[3]);
}

fn compress(h: ptr<function, array<u32, 8>>, m: array<u32, 16>, t: u32, is_final: bool) {
    var v: array<u32, 16>;
    for (var i = 0u; i < 8u; i++) {
        v[i] = (*h)[i];
        v[i + 8u] = IV[i];
    }
    v[12] ^= t;

    if (is_final) {
        v[14] ^= ~0u;
    }

    for (var i = 0u; i < R; i++) {
        var s: array<u32, 16>;
        for (var j = 0u; j < 16u; j++) {
            s[j] = SIGMA[i % 10][j];
        }
        g(&v, 0u, 4u, 8u, 12u, m[s[0]], m[s[1]]);
        g(&v, 1u, 5u, 9u, 13u, m[s[2]], m[s[3]]);
        g(&v, 2u, 6u, 10u, 14u, m[s[4]], m[s[5]]);
        g(&v, 3u, 7u, 11u, 15u, m[s[6]], m[s[7]]);

        g(&v, 0u, 5u, 10u, 15u, m[s[8]], m[s[9]]);
        g(&v, 1u, 6u, 11u, 12u, m[s[10]], m[s[11]]);
        g(&v, 2u, 7u, 8u, 13u, m[s[12]], m[s[13]]);
        g(&v, 3u, 4u, 9u, 14u, m[s[14]], m[s[15]]);
    }

    for (var i = 0u; i < 8u; i++) {
        (*h)[i] = (*h)[i] ^ v[i] ^ v[i + 8u];
    }
}

fn hex_nibble(n: u32) -> u32 {
    if (n < 10u) { return n + 0x30u; }
    return n - 10u + 0x61u;
}

fn blake2s(input_len: u32, nonce_byte_offset: u32, nonce: array<u32, 8>) -> array<u32, 8> {
    var h = IV;
    h[0] ^= 0x01010020u;

    if (input_len == 0u) {
        var m: array<u32, 16>;
        compress(&h, m, 0u, true);
        return h;
    }

    let nonce_byte_end = nonce_byte_offset + 64u;
    var t: u32 = 0u;
    var offset: u32 = 0u;

    loop {
        var block: array<u32, 16>;
        let remaining = input_len - offset;
        let chunk_size = min(remaining, 64u);
        let is_last = chunk_size == remaining;

        for (var i = 0u; i < 16u; i++) {
            let word_idx = offset / 4u + i;
            let word_byte_start = word_idx * 4u;

            if (word_byte_start + 4u <= nonce_byte_offset || word_byte_start >= nonce_byte_end) {
                if (word_idx >= arrayLength(&input)) {
                    block[i] = 0u;
                } else {
                    block[i] = input[word_idx];
                }
            } else {
                var word: u32 = 0u;
                for (var b = 0u; b < 4u; b++) {
                    let byte_pos = word_byte_start + b;
                    var byte_val: u32;
                    if (byte_pos >= nonce_byte_offset && byte_pos < nonce_byte_end) {
                        let idx = byte_pos - nonce_byte_offset;
                        let raw_byte_idx = idx / 2u;
                        let nonce_word = nonce[raw_byte_idx / 4u];
                        let byte_in_word = raw_byte_idx % 4u;
                        let raw_byte = (nonce_word >> (byte_in_word * 8u)) & 0xFFu;
                        if (idx % 2u == 0u) {
                            byte_val = hex_nibble((raw_byte >> 4u) & 0xFu);
                        } else {
                            byte_val = hex_nibble(raw_byte & 0xFu);
                        }
                    } else {
                        let src_word = input[byte_pos / 4u];
                        byte_val = (src_word >> ((byte_pos % 4u) * 8u)) & 0xFFu;
                    }
                    word |= byte_val << (b * 8u);
                }
                block[i] = word;
            }
        }

        t += chunk_size;
        compress(&h, block, t, is_last);

        if (is_last) {
            break;
        }
        offset += 64u;
    }

    return h;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (result[8] != 0u) { return; }

    let thread_id = gid.x;

    var nonce: array<u32, 8>;
    for (var i = 0u; i < 8u; i++) {
        nonce[i] = result[i];
    }
    nonce[7] += thread_id + params.batch_offset;

    let input_len = params.input_len;
    let nonce_byte_offset = params.nonce_byte_offset;

    let hash = blake2s(input_len, nonce_byte_offset, nonce);

    for (var i = 0u; i < 8u; i++) {
        for (var b = 0u; b < 4u; b++) {
            let shift = b * 8u;
            let hb = (hash[i] >> shift) & 0xFFu;
            let db = (difficulty[i] >> shift) & 0xFFu;
            if (hb < db) {
                for (var j = 0u; j < 8u; j++) {
                    result[j] = nonce[j];
                }
                result[8] = 1u;
                return;
            }
            if (hb > db) { return; }
        }
    }
}
