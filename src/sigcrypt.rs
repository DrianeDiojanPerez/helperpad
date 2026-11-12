//! SigString encryption, matching `com.topaz.sigplus.util.SigCrypt` in SigPlus Java 2.68:
//! mode 1 is DES, 2 is SAFER SK-64, 3 is SAFER K-64, and 4 is DES with a SHA-256 receipt.

use des::Des;
use des::cipher::{BlockEncrypt, KeyInit};
use md5::{Digest, Md5};
use sha2::Sha256;

const BLOCK: usize = 8;
const ENCRYPTED_TAG: [u8; 4] = [0xFF; 4];
const RECEIPT_CHUNK: usize = 512;
const SAFER_ROUNDS: usize = 13;

/// Wraps a clear SigString: the encrypted tag, a receipt of the clear bytes, the block
/// count, the padding in the last block, then the blocks. `None` for a mode Topaz doesn't have.
pub fn encrypt(clear: &[u8], mode: u64, key_text: &str) -> Option<Vec<u8>> {
    let key = short_key(key_text);
    let cipher = match mode {
        1 | 4 => Cipher::Des(Des::new(&key.into())),
        2 | 3 => Cipher::Safer(Box::new(Safer::new(&key, mode == 2))),
        _ => return None,
    };
    let receipt = if mode == 4 { fold(&Sha256::digest(clear)) } else { fold(&last_chunk_md5(clear)) };

    let blocks = clear.len().div_ceil(BLOCK);
    let padding = blocks * BLOCK - clear.len();

    let mut out = Vec::with_capacity(16 + blocks * BLOCK);
    out.extend(ENCRYPTED_TAG);
    out.extend(receipt.to_le_bytes());
    out.extend((blocks as u32).to_le_bytes());
    out.extend((padding as u32).to_le_bytes());
    for chunk in clear.chunks(BLOCK) {
        let mut block = [0u8; BLOCK];
        block[..chunk.len()].copy_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        out.extend(block);
    }
    Some(out)
}

enum Cipher {
    Des(Des),
    Safer(Box<Safer>),
}

impl Cipher {
    fn encrypt_block(&self, block: &mut [u8; BLOCK]) {
        match self {
            Self::Des(des) => des.encrypt_block(block.into()),
            Self::Safer(safer) => safer.encrypt_block(block),
        }
    }
}

/// `autoKeyData` hashes the key text with MD5, and `make40BitKey` keeps five bits
/// of each of the first eight bytes.
fn short_key(key_text: &str) -> [u8; BLOCK] {
    let hash = Md5::digest(key_text.as_bytes());
    std::array::from_fn(|i| hash[i] & 0x1F)
}

/// `calcHashForFileBuffer` starts a new MD5 for every 512 bytes and returns the last one,
/// so only the final chunk counts.
fn last_chunk_md5(clear: &[u8]) -> [u8; 16] {
    clear.chunks(RECEIPT_CHUNK).last().map_or([0; 16], |chunk| Md5::digest(chunk).into())
}

/// `convertHashToReceipt`: XORs the first 16 hash bytes into four, first byte highest.
fn fold(hash: &[u8]) -> u32 {
    let byte = |i: usize| hash[i] ^ hash[i + 4] ^ hash[i + 8] ^ hash[i + 12];
    u32::from_be_bytes([byte(0), byte(1), byte(2), byte(3)])
}

/// SAFER with a 64-bit key and 13 rounds, as Topaz's copy of the Cryptix cipher runs it.
/// The strong key schedule is SAFER SK-64, the weak one SAFER K-64.
struct Safer {
    exp: [u8; 256],
    log: [u8; 256],
    keys: [u8; BLOCK * (1 + 2 * SAFER_ROUNDS)],
}

impl Safer {
    fn new(key: &[u8; BLOCK], strong: bool) -> Self {
        let mut exp = [0u8; 256];
        let mut log = [0u8; 256];
        let mut power = 1u32;
        for i in 0..256 {
            exp[i] = power as u8;
            log[exp[i] as usize] = i as u8;
            power = power * 45 % 257;
        }

        let mut ka = [0u8; 9];
        let mut kb = [0u8; 9];
        let mut keys = [0u8; BLOCK * (1 + 2 * SAFER_ROUNDS)];
        for j in 0..BLOCK {
            ka[j] = key[j].rotate_left(5);
            ka[8] ^= ka[j];
            kb[j] = key[j];
            kb[8] ^= kb[j];
            keys[j] = key[j];
        }

        let bias = |index: usize| exp[exp[index] as usize];
        for round in 1..=SAFER_ROUNDS {
            for j in 0..9 {
                ka[j] = ka[j].rotate_left(6);
                kb[j] = kb[j].rotate_left(6);
            }
            let base = BLOCK * (2 * round - 1);
            for j in 0..BLOCK {
                let (a, b) = match strong {
                    true => (ka[(j + 2 * round - 1) % 9], kb[(j + 2 * round) % 9]),
                    false => (ka[j], kb[j]),
                };
                keys[base + j] = a.wrapping_add(bias(18 * round + j + 1));
                keys[base + BLOCK + j] = b.wrapping_add(bias(18 * round + j + 10));
            }
        }

        Self { exp, log, keys }
    }

    fn encrypt_block(&self, block: &mut [u8; BLOCK]) {
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *block;
        let exp = |value: u8| self.exp[value as usize];
        let log = |value: u8| self.log[value as usize];
        let mut keys = self.keys.iter().copied();
        let mut key = || keys.next().unwrap();

        for _ in 0..SAFER_ROUNDS {
            a ^= key();
            b = b.wrapping_add(key());
            c = c.wrapping_add(key());
            d ^= key();
            e ^= key();
            f = f.wrapping_add(key());
            g = g.wrapping_add(key());
            h ^= key();

            a = exp(a).wrapping_add(key());
            b = log(b) ^ key();
            c = log(c) ^ key();
            d = exp(d).wrapping_add(key());
            e = exp(e).wrapping_add(key());
            f = log(f) ^ key();
            g = log(g) ^ key();
            h = exp(h).wrapping_add(key());

            b = b.wrapping_add(a);
            a = a.wrapping_add(b);
            d = d.wrapping_add(c);
            c = c.wrapping_add(d);
            f = f.wrapping_add(e);
            e = e.wrapping_add(f);
            h = h.wrapping_add(g);
            g = g.wrapping_add(h);

            c = c.wrapping_add(a);
            a = a.wrapping_add(c);
            g = g.wrapping_add(e);
            e = e.wrapping_add(g);
            d = d.wrapping_add(b);
            b = b.wrapping_add(d);
            h = h.wrapping_add(f);
            f = f.wrapping_add(h);

            e = e.wrapping_add(a);
            a = a.wrapping_add(e);
            f = f.wrapping_add(b);
            b = b.wrapping_add(f);
            g = g.wrapping_add(c);
            c = c.wrapping_add(g);
            h = h.wrapping_add(d);
            d = d.wrapping_add(h);

            (b, c, d, e, f, g) = (e, b, f, c, g, d);
        }

        *block = [
            a ^ key(),
            b.wrapping_add(key()),
            c.wrapping_add(key()),
            d ^ key(),
            e ^ key(),
            f.wrapping_add(key()),
            g.wrapping_add(key()),
            h ^ key(),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "secret key 123";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect()
    }

    /// Expected values come from SigPlus Java 2.68 given the same clear SigString and key.
    #[test]
    fn matches_topaz_for_every_mode() {
        let clear = b"2\r\n1\r\n10 20\r\n30 40\r\n0\r\n";
        let expected = [
            (1, "FFFFFFFF39910F870300000001000000D8EB2A4D49F408D1357A882216C8CEEC481AFD9B7113FB02"),
            (2, "FFFFFFFF39910F87030000000100000005D4112840419008A3492DDD73C73BF9CBD7552541D354B9"),
            (3, "FFFFFFFF39910F8703000000010000005E8C1096C4B9096BDC43A6AF76719170964F8639D1153B72"),
            (4, "FFFFFFFF8885068D0300000001000000D8EB2A4D49F408D1357A882216C8CEEC481AFD9B7113FB02"),
        ];

        for (mode, hex_string) in expected {
            assert_eq!(hex(&encrypt(clear, mode, KEY).unwrap()), hex_string, "mode {mode}");
        }
    }

    #[test]
    fn an_empty_key_still_encrypts() {
        let clear = b"2\r\n1\r\n10 20\r\n30 40\r\n0\r\n";

        assert_eq!(
            hex(&encrypt(clear, 1, "").unwrap()),
            "FFFFFFFF39910F87030000000100000023DFE52AE0FD4965B54AC69E3A122ACC146646F44E83CADF"
        );
    }

    #[test]
    fn the_md5_receipt_covers_only_the_last_512_bytes() {
        let points: String = (0..80).map(|i| format!("{} {}\r\n", 100 + i, 200 + i)).collect();
        let clear = format!("80\r\n1\r\n{points}0\r\n");
        assert_eq!(clear.len(), 730);

        let md5_receipt = encrypt(clear.as_bytes(), 1, KEY).unwrap();
        let sha2_receipt = encrypt(clear.as_bytes(), 4, KEY).unwrap();

        assert_eq!(hex(&md5_receipt[..16]), "FFFFFFFF37E216A95C00000006000000");
        assert_eq!(hex(&sha2_receipt[..16]), "FFFFFFFF808971815C00000006000000");
        assert_eq!(md5_receipt[16..], sha2_receipt[16..]);
    }

    #[test]
    fn refuses_modes_topaz_does_not_have() {
        assert!(encrypt(b"1", 0, KEY).is_none());
        assert!(encrypt(b"1", 5, KEY).is_none());
    }
}
