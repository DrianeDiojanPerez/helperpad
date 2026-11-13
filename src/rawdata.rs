//! `rawData`, matching `Capture.GetSignatureData` in Topaz.SigPlusExtLite.exe 3.2.17.2.
//!
//! The blob starts with a clear "$VPAD$2.0$" header (signer details, pad owner, size,
//! time). The point segments after it are AES-256-CBC encrypted, and the AES key and IV
//! are RSA-encrypted (PKCS#1 v1.5) to eSign Emcee's "SigExtract" certificate, so only
//! eSign Emcee can read them.

use aes::Aes256;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockEncryptMut, KeyIvInit};
use rand::RngCore;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use serde_json::Value;
use sha1::{Digest, Sha1};

use crate::sigstring::Logical;

const SIG_EXTRACT_KEY: &[u8] = include_bytes!("../assets/esign-emcee-sigextract.der");
const BLOB_ID: &str = "$VPAD$2.0$";
const SIGN_WIDTH: i32 = 2000;
const SIGN_HEIGHT: i32 = 600;

/// The T-L460-HSB-R has no USB serial and stays silent when asked for its model and
/// serial (Topaz's command 0), so these stand in for SigPlus's `TabletModelNumber()`
/// and `TabletSerialNumber()`.
const PAD_MODEL: &str = "T-L460";
const PAD_SERIAL: &str = "helperpad";

pub fn pad_name() -> String {
    format!("Topaz {PAD_MODEL}")
}

pub fn pad_serial_number() -> String {
    sha1_hex(&format!("{PAD_MODEL}-{PAD_SERIAL}"))
}

pub struct Signer {
    first_name: String,
    last_name: String,
    email: String,
    location: String,
}

impl Signer {
    pub fn from_request(request: &Value) -> Self {
        let text = |key: &str| request[key].as_str().unwrap_or_default().to_string();
        Self {
            first_name: text("firstName"),
            last_name: text("lastName"),
            email: text("eMail"),
            location: text("location"),
        }
    }
}

pub fn encrypted(strokes: &[Vec<Logical>], signer: &Signer, now: i64, utc_offset: i32) -> Result<String, String> {
    let key = RsaPublicKey::from_public_key_der(SIG_EXTRACT_KEY).map_err(|e| e.to_string())?;
    encrypted_with_key(strokes, signer, now, utc_offset, &key)
}

fn encrypted_with_key(
    strokes: &[Vec<Logical>],
    signer: &Signer,
    now: i64,
    utc_offset: i32,
    key: &RsaPublicKey,
) -> Result<String, String> {
    let mut rng = rand::thread_rng();
    let mut session = [0u8; 48];
    rng.fill_bytes(&mut session);
    let (aes_key, iv) = session.split_at(32);

    let wrapped = key.encrypt(&mut rng, Pkcs1v15Encrypt, &session).map_err(|e| e.to_string())?;

    let points = point_data(&segments(strokes));
    let length = points.len();
    let mut buffer = points;
    buffer.resize(length + 16 - length % 16, 0);
    let ciphertext = cbc::Encryptor::<Aes256>::new_from_slices(aes_key, iv)
        .map_err(|e| e.to_string())?
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, length)
        .map_err(|e| e.to_string())?
        .to_vec();

    let mut blob = header(signer, now, utc_offset);
    put_bytes(&mut blob, &wrapped);
    put_bytes(&mut blob, &ciphertext);

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(blob))
}

/// `rawDataFormat: "SIG"`: the same points written as the plain-text .sig file.
pub fn sig_file(strokes: &[Vec<Logical>]) -> String {
    let segments = segments(strokes);
    let total: usize = segments.iter().map(Vec::len).sum();
    let mut text = format!("{total}\r\n{}\r\n", segments.len());

    for point in segments.iter().flatten() {
        text.push_str(&format!("{} {}\r\n", point[0], point[1]));
    }
    let mut start = 0;
    for segment in &segments {
        text.push_str(&format!("{start}\r\n"));
        start += segment.len();
    }
    text
}

fn header(signer: &Signer, now: i64, utc_offset: i32) -> Vec<u8> {
    let owner = format!("{} ({})", sha1_hex(&pad_serial_number()), pad_name());
    let mut blob = Vec::new();

    for text in [BLOB_ID, &signer.first_name, &signer.last_name, &signer.email, &signer.location, &owner] {
        put_bytes(&mut blob, text.as_bytes());
    }
    blob.extend_from_slice(&SIGN_WIDTH.to_le_bytes());
    blob.extend_from_slice(&SIGN_HEIGHT.to_le_bytes());
    blob.extend_from_slice(&now.to_le_bytes());
    blob.extend_from_slice(&utc_offset.to_le_bytes());
    blob
}

fn point_data(segments: &[Vec<[i32; 4]>]) -> Vec<u8> {
    let mut data = (segments.len() as i32).to_le_bytes().to_vec();
    for segment in segments {
        data.extend_from_slice(&(segment.len() as i32).to_le_bytes());
        for value in segment.iter().flatten() {
            data.extend_from_slice(&value.to_le_bytes());
        }
    }
    data
}

/// Splits points into segments of [x, y, pressure, ms since the first point] exactly as
/// Topaz's loop does, including its quirk of leaving the first stroke's last point out.
fn segments(strokes: &[Vec<Logical>]) -> Vec<Vec<[i32; 4]>> {
    let points: Vec<(Logical, bool)> = strokes
        .iter()
        .flat_map(|stroke| stroke.iter().enumerate().map(move |(i, p)| (*p, i + 1 < stroke.len())))
        .collect();

    let start = points.first().map_or(0, |(p, _)| p.t);
    let mut slots = vec![[0i32; 4]; points.len()];
    let mut segments = Vec::new();
    let (mut count, mut slot) = (0, 0);

    for (i, (point, touching)) in points.iter().enumerate() {
        if i == 0 && !touching {
            continue;
        }
        let touching = *touching && i + 1 < points.len();

        slots[slot] = [point.x, point.y, 0, point.t.saturating_sub(start) as i32];
        slot += 1;
        if !touching {
            segments.push(slots[..count].to_vec());
            count = 0;
            slot = 0;
        }
        count += 1;
    }

    segments
}

fn put_bytes(blob: &mut Vec<u8>, bytes: &[u8]) {
    blob.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
    blob.extend_from_slice(bytes);
}

fn sha1_hex(input: &str) -> String {
    Sha1::digest(input.as_bytes()).iter().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbc::cipher::BlockDecryptMut;
    use rsa::RsaPrivateKey;
    use serde_json::json;

    fn stroke(points: &[(i32, i32, u32)]) -> Vec<Logical> {
        points.iter().map(|&(x, y, t)| Logical { x, y, t }).collect()
    }

    fn take<'a>(blob: &'a [u8], at: &mut usize, len: usize) -> &'a [u8] {
        *at += len;
        &blob[*at - len..*at]
    }

    fn int(blob: &[u8], at: &mut usize) -> i32 {
        i32::from_le_bytes(take(blob, at, 4).try_into().unwrap())
    }

    #[test]
    fn segments_follow_topaz_including_the_first_stroke_quirk() {
        let strokes = [stroke(&[(1, 1, 10), (2, 2, 20), (3, 3, 30)]), stroke(&[(4, 4, 40), (5, 5, 50)])];

        assert_eq!(
            segments(&strokes),
            vec![
                vec![[1, 1, 0, 0], [2, 2, 0, 10]],
                vec![[4, 4, 0, 30], [5, 5, 0, 40]],
            ]
        );
    }

    #[test]
    fn writes_sig_text() {
        let strokes = [stroke(&[(1, 1, 0), (2, 2, 0), (3, 3, 0)]), stroke(&[(4, 4, 0), (5, 5, 0)])];

        assert_eq!(sig_file(&strokes), "4\r\n2\r\n1 1\r\n2 2\r\n4 4\r\n5 5\r\n0\r\n2\r\n");
    }

    #[test]
    fn the_embedded_sig_extract_key_loads() {
        let key = RsaPublicKey::from_public_key_der(SIG_EXTRACT_KEY).unwrap();
        assert_eq!(rsa::traits::PublicKeyParts::size(&key), 128);
    }

    #[test]
    fn blob_decrypts_back_to_the_points_with_the_matching_private_key() {
        let private = RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        let signer = Signer::from_request(&json!({ "firstName": "Ann", "location": "Office" }));
        let strokes = [stroke(&[(1, 1, 0), (2, 2, 5)]), stroke(&[(7, 8, 9), (9, 9, 12)])];

        let encoded = encrypted_with_key(&strokes, &signer, 1_800_000_000, -21600, &private.to_public_key()).unwrap();
        use base64::Engine;
        let blob = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();

        let mut at = 0;
        let texts: Vec<String> = (0..6)
            .map(|_| {
                let len = int(&blob, &mut at) as usize;
                String::from_utf8(take(&blob, &mut at, len).to_vec()).unwrap()
            })
            .collect();
        assert_eq!(texts[..5], [BLOB_ID, "Ann", "", "", "Office"]);
        assert!(texts[5].ends_with(&format!(" ({})", pad_name())));
        assert_eq!((int(&blob, &mut at), int(&blob, &mut at)), (SIGN_WIDTH, SIGN_HEIGHT));
        assert_eq!(i64::from_le_bytes(take(&blob, &mut at, 8).try_into().unwrap()), 1_800_000_000);
        assert_eq!(int(&blob, &mut at), -21600);

        let wrapped_len = int(&blob, &mut at) as usize;
        let session = private.decrypt(Pkcs1v15Encrypt, take(&blob, &mut at, wrapped_len)).unwrap();
        let cipher_len = int(&blob, &mut at) as usize;
        let mut ciphertext = take(&blob, &mut at, cipher_len).to_vec();
        assert_eq!(at, blob.len());

        let plain = cbc::Decryptor::<Aes256>::new_from_slices(&session[..32], &session[32..])
            .unwrap()
            .decrypt_padded_mut::<Pkcs7>(&mut ciphertext)
            .unwrap();
        assert_eq!(plain, point_data(&segments(&strokes)));
    }
}
