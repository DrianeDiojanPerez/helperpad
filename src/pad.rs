use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const X_MIN: f32 = 400.0;
pub const X_MAX: f32 = 2400.0;
pub const Y_MIN: f32 = 350.0;
pub const Y_MAX: f32 = 950.0;

const TOPAZ_HID_ID: &str = "000006A8";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    /// Milliseconds since capture started.
    pub t: u32,
}

#[derive(Debug, PartialEq)]
pub enum PenEvent {
    Down(Point),
    Up,
}

#[derive(Clone, Default)]
pub struct Ink {
    pub strokes: Vec<Vec<Point>>,
    pen_down: bool,
}

impl Ink {
    pub fn apply(&mut self, event: PenEvent) {
        match event {
            PenEvent::Down(point) => {
                if !self.pen_down {
                    self.strokes.push(Vec::new());
                    self.pen_down = true;
                }
                self.strokes.last_mut().unwrap().push(point);
            }
            PenEvent::Up => self.pen_down = false,
        }
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
        self.pen_down = false;
    }

    pub fn point_count(&self) -> usize {
        self.strokes.iter().map(Vec::len).sum()
    }
}

pub fn find() -> Option<PathBuf> {
    fs::read_dir("/sys/class/hidraw")
        .ok()?
        .flatten()
        .find(|entry| {
            fs::read_to_string(entry.path().join("device/uevent"))
                .is_ok_and(|uevent| uevent.contains(TOPAZ_HID_ID))
        })
        .map(|entry| PathBuf::from("/dev").join(entry.file_name()))
}

/// Each HID report is a length byte followed by one Topaz point:
/// a status byte (0x80 sync, 0x40 pen touching) and X/Y as two 7-bit halves.
pub fn decode(report: &[u8]) -> Option<PenEvent> {
    let &[_, status, x_lo, x_hi, y_lo, y_hi, ..] = report else {
        return None;
    };

    if status & 0x80 == 0 || [x_lo, x_hi, y_lo, y_hi].iter().any(|b| b & 0x80 != 0) {
        return None;
    }

    if status & 0x40 == 0 {
        return Some(PenEvent::Up);
    }

    Some(PenEvent::Down(Point {
        x: f32::from(u16::from(x_lo) | u16::from(x_hi) << 7),
        y: f32::from(u16::from(y_lo) | u16::from(y_hi) << 7),
        t: 0,
    }))
}

pub fn listen(path: &Path, mut on_event: impl FnMut(PenEvent)) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut report = [0u8; 64];
    let started = Instant::now();

    loop {
        let len = file.read(&mut report)?;
        if len == 0 {
            return Ok(());
        }
        if let Some(mut event) = decode(&report[..len]) {
            if let PenEvent::Down(point) = &mut event {
                point.t = started.elapsed().as_millis() as u32;
            }
            on_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_reports_captured_from_a_t_l460() {
        assert_eq!(
            decode(&[0x05, 0xc1, 0x62, 0x07, 0x0b, 0x04, 0x08, 0x4c]),
            Some(PenEvent::Down(Point { x: 994.0, y: 523.0, t: 0 }))
        );
        assert_eq!(
            decode(&[0x05, 0x80, 0x44, 0x08, 0x77, 0x04, 0x08, 0x4c]),
            Some(PenEvent::Up)
        );
        assert_eq!(decode(&[0x05, 0x41, 0x00, 0x00, 0x00, 0x00]), None);
        assert_eq!(decode(&[0x05, 0xc1]), None);
    }

    #[test]
    fn splits_strokes_on_pen_up() {
        let mut ink = Ink::default();
        let point = Point { x: 1.0, y: 1.0, t: 0 };
        ink.apply(PenEvent::Down(point));
        ink.apply(PenEvent::Down(point));
        ink.apply(PenEvent::Up);
        ink.apply(PenEvent::Down(point));

        assert_eq!(ink.strokes.len(), 2);
        assert_eq!(ink.point_count(), 3);
    }
}
