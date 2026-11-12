use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::log;
use crate::pad::{self, Point};
use crate::rawdata;
use crate::render::{self, ImageOptions};
use crate::sigcrypt;
use crate::sigstring;

/// Reported as the SDK version. The extension treats anything at 3.0 or above
/// as a host that keeps one connection open across commands.
const SPEL_VERSION: &str = "3.1.0.0";

pub fn read_message(input: &mut impl Read) -> io::Result<Option<Value>> {
    let mut length = [0u8; 4];
    if let Err(e) = input.read_exact(&mut length) {
        return if e.kind() == io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(e) };
    }

    let mut body = vec![0u8; u32::from_ne_bytes(length) as usize];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(io::Error::other)
}

pub fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)?;
    output.write_all(&(body.len() as u32).to_ne_bytes())?;
    output.write_all(&body)?;
    output.flush()
}

pub fn serve() -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    while let Some(request) = read_message(&mut stdin)? {
        let command = request["metadata"]["command"].as_str().unwrap_or_default().to_string();
        log::write(&format!("request {command}"));

        let mut response = respond(&command, &request);
        response["metadata"] = request["metadata"].clone();
        write_message(&mut stdout, &response)?;

        log::write(&format!(
            "response {command}: responseType={} isSigned={} errorMsg={}",
            response["responseType"], response["isSigned"], response["errorMsg"]
        ));

        if command == "Disconnect" {
            break;
        }
    }

    Ok(())
}

fn respond(command: &str, request: &Value) -> Value {
    match command {
        "SignatureCapture" => capture(request),
        "GetVersionInfo" => json!({ "responseType": 3, "spelVersion": SPEL_VERSION, "status": 1, "errorMsg": "" }),
        "GetDeviceStatus" => json!({
            "responseType": 1,
            "deviceStatus": i32::from(pad::find().is_some()),
            "errorMsg": "",
        }),
        "Connect" => json!({ "responseType": 24, "status": 1, "errorMsg": "" }),
        "Disconnect" => json!({ "responseType": 53, "status": 1, "errorMsg": "" }),
        "GetActiveXVersionInfo" => json!({ "responseType": 5, "activeXVersion": SPEL_VERSION, "status": 1, "errorMsg": "" }),
        _ => json!({ "responseType": -1, "errorMsg": "is not supported by helperpad." }),
    }
}

fn not_signed(error: &str) -> Value {
    json!({
        "responseType": 0,
        "isSigned": false,
        "imageData": "",
        "rawData": "",
        "sigString": "",
        "padInfo": "",
        "errorMsg": error,
    })
}

fn capture(request: &Value) -> Value {
    let Some(device) = pad::find() else {
        return not_signed("No Topaz signature pad found. Check that it is plugged in.");
    };

    let min_points = request["minSigPoints"].as_u64().unwrap_or(25);

    let strokes = match run_signing_window(&device.to_string_lossy(), min_points) {
        Ok(Some(strokes)) => strokes,
        Ok(None) => return not_signed("Signing was cancelled."),
        Err(e) => {
            log::write(&format!("signing window failed: {e}"));
            return not_signed(&format!("helperpad could not open the signing window: {e}"));
        }
    };

    let image = match render::render(&strokes, &ImageOptions::from_request(request)) {
        Ok(image) => STANDARD.encode(image),
        Err(e) => return not_signed(&format!("helperpad could not draw the signature: {e}")),
    };

    json!({
        "responseType": 0,
        "isSigned": true,
        "imageData": image,
        "rawData": raw_data(&strokes, request),
        "sigString": sig_string(&strokes, request),
        "padInfo": format!("{}{}", rawdata::pad_name(), rawdata::pad_serial_number()),
        "errorMsg": "",
    })
}

fn integer(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.trim().parse().ok())
}

fn sig_string(strokes: &[Vec<Point>], request: &Value) -> String {
    let compression = integer(&request["sigCompressionMode"]).unwrap_or(0).min(8) as u32;
    let clear = sigstring::encode(strokes, compression);

    let encryption = integer(&request["encryptionMode"]).unwrap_or(0);
    if encryption == 0 {
        return sigstring::hex(&clear);
    }

    let key = request["encryptionKey"].as_str().unwrap_or_default();
    match sigcrypt::encrypt(&clear, encryption, key) {
        Some(encrypted) => sigstring::hex(&encrypted),
        None => {
            log::write(&format!("sigString left empty: encryptionMode {encryption} is not supported"));
            String::new()
        }
    }
}

/// Topaz defaults to "ENC", the eSign Emcee encrypted blob; "SIG" is the plain .sig text.
fn raw_data(strokes: &[Vec<Point>], request: &Value) -> String {
    let logical = sigstring::logical_strokes(strokes);
    if request["rawDataFormat"].as_str() == Some("SIG") {
        return rawdata::sig_file(&logical);
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let signer = rawdata::Signer::from_request(request);
    rawdata::encrypted(&logical, &signer, now, standard_utc_offset()).unwrap_or_else(|e| {
        log::write(&format!("rawData left empty: {e}"));
        String::new()
    })
}

/// .NET's `TimeZoneInfo.Local.BaseUtcOffset`: the offset without daylight saving,
/// which is the smaller of the offsets half a year apart.
fn standard_utc_offset() -> i32 {
    let offset_at = |time: libc::time_t| {
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let ok = unsafe { !libc::localtime_r(&time, &mut tm).is_null() };
        if ok { tm.tm_gmtoff as i32 } else { 0 }
    };

    let now = unsafe { libc::time(std::ptr::null_mut()) };
    offset_at(now).min(offset_at(now + 182 * 24 * 3600))
}

/// The window runs in a child process because a process can only open one GUI event loop,
/// while this host stays alive for many signatures.
fn run_signing_window(device: &str, min_points: u64) -> Result<Option<Vec<Vec<Point>>>, String> {
    let output = Command::new(crate::own_path()?)
        .args(["--sign", device, &min_points.to_string()])
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| e.to_string())?;

    let result: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| format!("the window exited with {}", output.status))?;

    if let Some(error) = result["error"].as_str() {
        return Err(error.to_string());
    }

    let Some(strokes) = result["strokes"].as_array() else {
        return Ok(None);
    };

    Ok(Some(
        strokes
            .iter()
            .map(|stroke| {
                stroke
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| {
                        Some(Point {
                            x: p[0].as_f64()? as f32,
                            y: p[1].as_f64()? as f32,
                            t: p[2].as_f64().unwrap_or(0.0) as u32,
                        })
                    })
                    .collect()
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_messages_like_chrome() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, &json!({ "a": 1 })).unwrap();

        assert_eq!(&buffer[..4], &7u32.to_ne_bytes());
        assert_eq!(read_message(&mut buffer.as_slice()).unwrap(), Some(json!({ "a": 1 })));
        assert_eq!(read_message(&mut [].as_slice()).unwrap(), None);
    }

    #[test]
    fn encrypts_the_sig_string_when_the_page_asks() {
        let strokes = vec![vec![Point { x: 500.0, y: 400.0, t: 0 }, Point { x: 504.0, y: 408.0, t: 5 }]];
        let clear = sig_string(&strokes, &json!({}));
        let encrypted = sig_string(&strokes, &json!({ "encryptionMode": "1", "encryptionKey": "abc" }));

        assert!(clear.starts_with("36"));
        assert!(encrypted.starts_with("FFFFFFFF"));
        assert_ne!(encrypted, sig_string(&strokes, &json!({ "encryptionMode": 1, "encryptionKey": "other" })));
        assert_eq!(sig_string(&strokes, &json!({ "encryptionMode": 9 })), "");
    }

    #[test]
    fn answers_version_info_with_a_3x_sdk_version() {
        let response = respond("GetVersionInfo", &json!({}));

        assert_eq!(response["responseType"], 3);
        assert_eq!(response["status"], 1);
        assert!(response["spelVersion"].as_str().unwrap().starts_with("3."));
    }

    #[test]
    fn flags_unknown_commands_as_unsupported() {
        assert_eq!(respond("LCDWriteImage", &json!({}))["responseType"], -1);
    }
}
