//! Topaz SigString, matching `com.topaz.sigplus.util.SigWriter` in SigPlus Java 2.68
//! and byte for byte what the Windows SigPlus control writes: no pressure or time data,
//! and an empty timestamp and annotation block at the end. `sigcrypt` adds the encryption.

use crate::pad::{Point, X_MAX, X_MIN, Y_MAX, Y_MIN};

const LOGICAL_X_SIZE: i32 = 2000;
const LOGICAL_Y_SIZE: i32 = 600;
const FILTER_POINTS: usize = 4;

/// `SigWriter.saveSigInfo` with nothing set: a length byte and eight shorts for the
/// timestamp, then the same for the annotation. The Windows control always writes it.
const EMPTY_SIG_INFO: [u8; 34] = [0; 34];

pub fn encode(strokes: &[Vec<Point>], compression_mode: u32) -> Vec<u8> {
    write(&logical_strokes(strokes), compression_mode)
}

fn write(strokes: &[Vec<Logical>], compression_mode: u32) -> Vec<u8> {
    let mut bytes = if compression_mode == 0 {
        uncompressed(strokes)
    } else {
        compressed(strokes, compression_mode as i32)
    };
    bytes.extend(EMPTY_SIG_INFO);
    bytes
}

/// A SigString travels as uppercase hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

/// A point in SigPlus logical coordinates (0..2000 x 0..600 for a 1x5 pad).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Logical {
    pub x: i32,
    pub y: i32,
    pub t: u32,
}

/// Scales pad coordinates the way `TabletInterface.scaleCoordData` does, then applies
/// its moving-average filter: the first point is raw, later points average the last
/// four samples, and lifting the pen adds four more averaged copies of the last sample.
pub fn logical_strokes(strokes: &[Vec<Point>]) -> Vec<Vec<Logical>> {
    let scale = |p: &Point| {
        let x = (p.x as i32 - X_MIN as i32) * LOGICAL_X_SIZE / (X_MAX - X_MIN) as i32;
        let y = (p.y as i32 - Y_MIN as i32) * LOGICAL_Y_SIZE / (Y_MAX - Y_MIN) as i32;
        ((0..LOGICAL_X_SIZE).contains(&x) && (0..LOGICAL_Y_SIZE).contains(&y)).then_some(Logical { x, y, t: p.t })
    };

    let mut out = Vec::new();
    for stroke in strokes {
        let mut current = Vec::new();
        for point in stroke {
            match scale(point) {
                Some(p) => current.push(p),
                None if !current.is_empty() => out.push(std::mem::take(&mut current)),
                None => {}
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }

    out.iter().map(|raw| filtered(raw)).collect()
}

fn filtered(raw: &[Logical]) -> Vec<Logical> {
    let mut window = [raw[0]; FILTER_POINTS];
    let mut next = 0;
    let mut average = |sample: Logical| {
        window[next] = sample;
        next = (next + 1) % FILTER_POINTS;
        let (x, y) = window.iter().fold((0, 0), |(x, y), p| (x + p.x, y + p.y));
        Logical { x: x / FILTER_POINTS as i32, y: y / FILTER_POINTS as i32, t: sample.t }
    };

    let mut points = vec![raw[0]];
    points.extend(raw[1..].iter().map(|p| average(*p)));
    let last = *raw.last().unwrap();
    points.extend((0..FILTER_POINTS).map(|_| average(last)));
    points
}

/// `SigWriter.writeSigUncompressed`: ASCII counts, every point, then the index where each stroke starts.
fn uncompressed(strokes: &[Vec<Logical>]) -> Vec<u8> {
    let total: usize = strokes.iter().map(Vec::len).sum();
    let mut text = format!("{total}\r\n{}\r\n", strokes.len());

    for p in strokes.iter().flatten() {
        text.push_str(&format!("{} {}\r\n", p.x, p.y));
    }
    let mut start = 0;
    for stroke in strokes {
        text.push_str(&format!("{start}\r\n"));
        start += stroke.len();
    }

    text.into_bytes()
}

/// `SigWriter.writeSigCompressed`: per stroke a point count, the first point as two
/// little-endian shorts, then one-byte deltas for every `mode`-th point plus the last.
fn compressed(strokes: &[Vec<Logical>], mode: i32) -> Vec<u8> {
    let mut out = Vec::new();
    let short = |out: &mut Vec<u8>, v: i32| out.extend_from_slice(&(v as u16).to_le_bytes());
    let delta = |out: &mut Vec<u8>, from: Logical, to: Logical| {
        out.push((to.x - from.x) as u8);
        out.push((to.y - from.y) as u8);
    };

    short(&mut out, strokes.len() as i32);
    for stroke in strokes {
        let count = stroke.len() as i32;
        short(&mut out, (count - 2) / mode + 2);

        let mut previous = stroke[0];
        short(&mut out, previous.x);
        short(&mut out, previous.y);

        for (offset, point) in stroke.iter().enumerate().take(stroke.len().saturating_sub(1)).skip(1) {
            if offset as i32 % mode == 0 {
                delta(&mut out, previous, *point);
                previous = *point;
            }
        }
        if count > 1 {
            delta(&mut out, previous, *stroke.last().unwrap());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(points: &[(f32, f32)]) -> Vec<Point> {
        points.iter().map(|&(x, y)| Point { x, y, t: 0 }).collect()
    }

    fn unhex(hex: &str) -> String {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as char)
            .collect()
    }

    #[test]
    fn writes_uncompressed_text_with_filtered_logical_points() {
        let hex = hex(&encode(&[stroke(&[(500.0, 400.0), (504.0, 408.0)])], 0));

        assert_eq!(
            unhex(&hex).trim_end_matches('\0'),
            "6\r\n1\r\n100 50\r\n101 52\r\n102 54\r\n103 56\r\n104 58\r\n104 58\r\n0\r\n"
        );
    }

    #[test]
    fn writes_compressed_deltas() {
        let hex = hex(&encode(&[stroke(&[(500.0, 400.0), (504.0, 408.0)])], 1));

        assert_eq!(hex, format!("010006006400320001020102010201020000{}", "00".repeat(34)));
    }

    /// Expected values were written by Topaz's Windows SigPlus control for the same two points.
    #[test]
    fn matches_the_windows_control() {
        let strokes = [vec![Logical { x: 10, y: 20, t: 0 }, Logical { x: 30, y: 40, t: 0 }]];

        assert_eq!(
            hex(&write(&strokes, 0)),
            format!("320D0A310D0A31302032300D0A33302034300D0A300D0A{}", "00".repeat(34))
        );
        assert_eq!(hex(&write(&strokes, 1)), format!("010002000A0014001414{}", "00".repeat(34)));
    }

    #[test]
    fn splits_strokes_that_leave_the_signing_area() {
        let hex = hex(&encode(&[stroke(&[(500.0, 400.0), (100.0, 400.0), (600.0, 400.0)])], 0));

        assert!(unhex(&hex).starts_with("10\r\n2\r\n"));
    }
}
