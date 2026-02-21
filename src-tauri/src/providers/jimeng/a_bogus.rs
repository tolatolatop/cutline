use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// SM3 (Chinese national standard hash, GB/T 32905-2016)
// Ported from TiktokDouyinCrawler/utils/a_bogus.js
// ---------------------------------------------------------------------------

const SM3_IV: [u32; 8] = [
    0x7380166F, 0x4914B2B9, 0x172442D7, 0xDA8A0600,
    0xA96F30BC, 0x163138AA, 0xE38DEE4D, 0xB0FB0E4E,
];

fn sm3_t(j: usize) -> u32 {
    if j < 16 { 0x79CC4519 } else { 0x7A879D8A }
}

fn sm3_ff(j: usize, x: u32, y: u32, z: u32) -> u32 {
    if j < 16 { x ^ y ^ z } else { (x & y) | (x & z) | (y & z) }
}

fn sm3_gg(j: usize, x: u32, y: u32, z: u32) -> u32 {
    if j < 16 { x ^ y ^ z } else { (x & y) | (!x & z) }
}

struct Sm3 {
    reg: [u32; 8],
    chunk: Vec<u8>,
    size: usize,
}

impl Sm3 {
    fn new() -> Self {
        Self { reg: SM3_IV, chunk: Vec::new(), size: 0 }
    }

    fn reset(&mut self) {
        self.reg = SM3_IV;
        self.chunk.clear();
        self.size = 0;
    }

    fn write_bytes(&mut self, data: &[u8]) {
        self.size += data.len();
        self.chunk.extend_from_slice(data);
        if self.chunk.len() >= 64 {
            let mut offset = 0;
            while offset + 64 <= self.chunk.len() {
                let block: [u8; 64] = self.chunk[offset..offset + 64].try_into().unwrap();
                self.compress(&block);
                offset += 64;
            }
            self.chunk = self.chunk[offset..].to_vec();
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 132];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4], block[i * 4 + 1], block[i * 4 + 2], block[i * 4 + 3],
            ]);
        }
        for i in 16..68 {
            let mut a = w[i - 16] ^ w[i - 9] ^ w[i - 3].rotate_left(15);
            a = a ^ a.rotate_left(15) ^ a.rotate_left(23);
            w[i] = a ^ w[i - 13].rotate_left(7) ^ w[i - 6];
        }
        for i in 0..64 {
            w[i + 68] = w[i] ^ w[i + 4];
        }

        let mut v = self.reg;
        for j in 0..64 {
            let ss1 = {
                let tmp = (v[0].rotate_left(12) as u64)
                    .wrapping_add(v[4] as u64)
                    .wrapping_add(sm3_t(j).rotate_left(j as u32 % 32) as u64);
                (tmp as u32).rotate_left(7)
            };
            let ss2 = ss1 ^ v[0].rotate_left(12);
            let tt1 = sm3_ff(j, v[0], v[1], v[2])
                .wrapping_add(v[3])
                .wrapping_add(ss2)
                .wrapping_add(w[j + 68]);
            let tt2 = sm3_gg(j, v[4], v[5], v[6])
                .wrapping_add(v[7])
                .wrapping_add(ss1)
                .wrapping_add(w[j]);

            v[3] = v[2];
            v[2] = v[1].rotate_left(9);
            v[1] = v[0];
            v[0] = tt1;
            v[7] = v[6];
            v[6] = v[5].rotate_left(19);
            v[5] = v[4];
            v[4] = tt2 ^ tt2.rotate_left(9) ^ tt2.rotate_left(17);
        }
        for (i, vi) in v.iter().enumerate() {
            self.reg[i] ^= vi;
        }
    }

    fn finalize(&mut self) -> [u8; 32] {
        let bit_len = (self.size as u64) * 8;
        self.chunk.push(0x80);
        while self.chunk.len() % 64 != 56 {
            self.chunk.push(0);
        }
        self.chunk.extend_from_slice(&bit_len.to_be_bytes());

        let chunks = self.chunk.clone();
        for blk in chunks.chunks_exact(64) {
            self.compress(blk.try_into().unwrap());
        }

        let mut out = [0u8; 32];
        for (i, &r) in self.reg.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&r.to_be_bytes());
        }
        self.reset();
        out
    }
}

fn sm3_hash(data: &[u8]) -> [u8; 32] {
    let mut h = Sm3::new();
    h.write_bytes(data);
    h.finalize()
}

fn sm3_hash_str(s: &str) -> [u8; 32] {
    sm3_hash(s.as_bytes())
}

fn sm3_double_hash_str(s: &str) -> [u8; 32] {
    let first = sm3_hash_str(s);
    sm3_hash(&first)
}

fn sm3_double_hash_bytes(b: &[u8]) -> [u8; 32] {
    let first = sm3_hash(b);
    sm3_hash(&first)
}

// ---------------------------------------------------------------------------
// RC4
// ---------------------------------------------------------------------------

fn rc4_encrypt(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    let mut s: Vec<u8> = (0..=255).collect();
    let mut j: usize = 0;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) % 256;
        s.swap(i, j);
    }

    let mut i: usize = 0;
    j = 0;
    plaintext.iter().map(|&b| {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        let t = (s[i] as usize + s[j] as usize) % 256;
        s[t] ^ b
    }).collect()
}

// ---------------------------------------------------------------------------
// Custom base64-like encoding tables
// ---------------------------------------------------------------------------

const S3: &[u8] = b"ckdp1h4ZKsUB80/Mfvw36XIgR25+WQAlEi7NLboqYTOPuzmFjJnryx9HVGDaStCe";
const S4: &[u8] = b"Dkdpgh2ZmsQB80/MfvV36XI1R45-WUAlEixNLwoqYTOPuzKFjJnry79HbGcaStCe";

fn result_encrypt(data: &[u8], table: &[u8]) -> Vec<u8> {
    let full_groups = data.len() / 3;
    let mut result = Vec::with_capacity(full_groups * 4 + 4);
    for g in 0..full_groups {
        let base = g * 3;
        let long_int = ((data[base] as u32) << 16)
            | ((data[base + 1] as u32) << 8)
            | (data[base + 2] as u32);
        result.push(table[((long_int >> 18) & 63) as usize]);
        result.push(table[((long_int >> 12) & 63) as usize]);
        result.push(table[((long_int >> 6) & 63) as usize]);
        result.push(table[(long_int & 63) as usize]);
    }
    result
}

fn result_encrypt_to_string(data: &[u8], table: &[u8]) -> String {
    String::from_utf8(result_encrypt(data, table)).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Random helpers
// ---------------------------------------------------------------------------

fn gener_random(random: u16, option: [u8; 2]) -> [u8; 4] {
    let r = random;
    [
        ((r as u8) & 170) | (option[0] & 85),
        ((r as u8) & 85) | (option[0] & 170),
        (((r >> 8) as u8) & 170) | (option[1] & 85),
        (((r >> 8) as u8) & 85) | (option[1] & 170),
    ]
}

fn generate_random_bytes() -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&gener_random(rng.gen_range(0..10000), [3, 45]));
    out.extend_from_slice(&gener_random(rng.gen_range(0..10000), [1, 0]));
    out.extend_from_slice(&gener_random(rng.gen_range(0..10000), [1, 5]));
    out
}

// ---------------------------------------------------------------------------
// Core payload generation
// ---------------------------------------------------------------------------

const WINDOW_ENV: &str = "1536|747|1536|834|0|30|0|0|1536|834|1536|864|1525|747|24|24|Win32";

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn generate_rc4_bb(url_params: &str, user_agent: &str) -> Vec<u8> {
    let start_time = now_ms();

    let url_hash = sm3_double_hash_str(&format!("{}{}", url_params, "cus"));
    let cus_hash = sm3_double_hash_bytes("cus".as_bytes());

    let ua_rc4 = rc4_encrypt(user_agent.as_bytes(), &[0, 1, 14]);
    let ua_encoded = result_encrypt(&ua_rc4, S3);
    let ua_hash = sm3_hash(&ua_encoded);

    let end_time = now_ms();

    let page_id: u32 = 6241;
    let aid: u32 = 6383;

    let mut b = std::collections::HashMap::<usize, u64>::new();
    b.insert(8, 3);
    b.insert(10, end_time);
    b.insert(16, start_time);
    b.insert(18, 44);

    let b20 = ((start_time >> 24) & 255) as u8;
    let b21 = ((start_time >> 16) & 255) as u8;
    let b22 = ((start_time >> 8) & 255) as u8;
    let b23 = (start_time & 255) as u8;
    let b24 = (start_time / 256 / 256 / 256 / 256) as u8;
    let b25 = (start_time / 256 / 256 / 256 / 256 / 256) as u8;

    let args: [u32; 3] = [0, 1, 14];
    let b26 = ((args[0] >> 24) & 255) as u8;
    let b27 = ((args[0] >> 16) & 255) as u8;
    let b28 = ((args[0] >> 8) & 255) as u8;
    let b29 = (args[0] & 255) as u8;
    let b30 = ((args[1] / 256) & 255) as u8;
    let b31 = (args[1] % 256) as u8;
    let b32 = ((args[1] >> 24) & 255) as u8;
    let b33 = ((args[1] >> 16) & 255) as u8;
    let b34 = ((args[2] >> 24) & 255) as u8;
    let b35 = ((args[2] >> 16) & 255) as u8;
    let b36 = ((args[2] >> 8) & 255) as u8;
    let b37 = (args[2] & 255) as u8;

    let b38 = url_hash[21];
    let b39 = url_hash[22];
    let b40 = cus_hash[21];
    let b41 = cus_hash[22];
    let b42 = ua_hash[23];
    let b43 = ua_hash[24];

    let b44 = ((end_time >> 24) & 255) as u8;
    let b45 = ((end_time >> 16) & 255) as u8;
    let b46 = ((end_time >> 8) & 255) as u8;
    let b47 = (end_time & 255) as u8;
    let b48 = 3u8; // b[8]
    let b49 = (end_time / 256 / 256 / 256 / 256) as u8;
    let b50 = (end_time / 256 / 256 / 256 / 256 / 256) as u8;

    let b52 = ((page_id >> 24) & 255) as u8;
    let b53 = ((page_id >> 16) & 255) as u8;
    let b54 = ((page_id >> 8) & 255) as u8;
    let b55 = (page_id & 255) as u8;
    let b57 = (aid & 255) as u8;
    let b58 = ((aid >> 8) & 255) as u8;
    let b59 = ((aid >> 16) & 255) as u8;
    let b60 = ((aid >> 24) & 255) as u8;

    let window_env_bytes: Vec<u8> = WINDOW_ENV.bytes().collect();
    let b65 = (window_env_bytes.len() & 255) as u8;
    let b66 = ((window_env_bytes.len() >> 8) & 255) as u8;
    let b70 = 0u8; // [].length & 255
    let b71 = 0u8; // [].length >> 8 & 255

    let b72 = 44u8 ^ b20 ^ b26 ^ b30 ^ b38 ^ b40 ^ b42 ^ b21 ^ b27 ^ b31 ^ b35 ^ b39
        ^ b41 ^ b43 ^ b22 ^ b28 ^ b32 ^ b36 ^ b23 ^ b29 ^ b33 ^ b37 ^ b44 ^ b45 ^ b46
        ^ b47 ^ b48 ^ b49 ^ b50 ^ b24 ^ b25 ^ b52 ^ b53 ^ b54 ^ b55 ^ b57 ^ b58 ^ b59
        ^ b60 ^ b65 ^ b66 ^ b70 ^ b71;

    let mut bb: Vec<u8> = vec![
        44,  b20, b52, b26, b30, b34, b58, b38, b40, b53, b42, b21, b27, b54, b55, b31,
        b35, b57, b39, b41, b43, b22, b28, b32, b60, b36, b23, b29, b33, b37, b44, b45,
        b59, b46, b47, b48, b49, b50, b24, b25, b65, b66, b70, b71,
    ];
    bb.extend_from_slice(&window_env_bytes);
    bb.push(b72);

    rc4_encrypt(&bb, &[121]) // key = 'y'
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn generate_a_bogus(url_search_params: &str, user_agent: &str) -> String {
    let mut payload = generate_random_bytes();
    payload.extend_from_slice(&generate_rc4_bb(url_search_params, user_agent));
    result_encrypt_to_string(&payload, S4) + "="
}

pub fn generate_ms_token(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIGKLMNOPQRSTUVWXYZabcdefghigklmnopqrstuvwxyz0123456789=";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len() - 1)] as char)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm3_known_vector_empty() {
        let hash = sm3_hash(b"");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b");
    }

    #[test]
    fn sm3_known_vector_abc() {
        let hash = sm3_hash(b"abc");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0");
    }

    #[test]
    fn rc4_roundtrip() {
        let plaintext = b"hello world";
        let key = b"secret";
        let encrypted = rc4_encrypt(plaintext, key);
        let decrypted = rc4_encrypt(&encrypted, key);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn ms_token_length_and_charset() {
        let token = generate_ms_token(128);
        assert_eq!(token.len(), 128);
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '='));
    }

    #[test]
    fn a_bogus_non_empty_and_ends_with_eq() {
        let result = generate_a_bogus(
            "device_platform=web&aid=513695&region=cn",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        );
        assert!(!result.is_empty());
        assert!(result.ends_with('='));
        assert!(result.len() > 50);
    }

    #[test]
    fn a_bogus_different_each_call() {
        let a = generate_a_bogus("test=1", "UA");
        let b = generate_a_bogus("test=1", "UA");
        assert_ne!(a, b, "a_bogus should differ due to randomness/timestamps");
    }

    #[test]
    fn result_encrypt_basic() {
        let data = b"abc";
        let encoded = result_encrypt_to_string(data, S4);
        assert!(!encoded.is_empty());
    }
}
