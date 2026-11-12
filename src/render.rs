use image::ExtendedColorType;
use image::codecs::jpeg::JpegEncoder;
use serde_json::Value;
use tiny_skia::{Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::pad::{Point, X_MAX, X_MIN, Y_MAX, Y_MIN};

pub struct ImageOptions {
    pub width: u32,
    pub height: u32,
    pub png: bool,
    pub transparent: bool,
    pub fit_to_signature: bool,
    pub pen_width: f32,
    pub color: [u8; 3],
}

impl ImageOptions {
    /// Reads the SignatureCapture fields from Topaz's integration guide.
    pub fn from_request(request: &Value) -> Self {
        let number = |key: &str, default: f64| {
            let value = &request[key];
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(default)
        };

        let width = number("imageX", 500.0).clamp(10.0, 4000.0) as u32;
        let height = number("imageY", 100.0).clamp(10.0, 4000.0) as u32;
        let thickness = number("penThickness", 1.0).clamp(1.0, 5.0) as f32;

        Self {
            width,
            height,
            png: number("imageFormat", 2.0) as i64 != 1,
            transparent: request["imageTransparency"].as_bool().unwrap_or(false),
            fit_to_signature: request["imageScaling"].as_bool().unwrap_or(false),
            pen_width: (height as f32 / 50.0).max(2.0) * thickness,
            color: parse_color(request["penColor"].as_str().unwrap_or("#000000")),
        }
    }
}

fn parse_color(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    let channel = |i: usize| hex.get(i..i + 2).and_then(|c| u8::from_str_radix(c, 16).ok());

    match (channel(0), channel(2), channel(4)) {
        (Some(r), Some(g), Some(b)) => [r, g, b],
        _ => [0, 0, 0],
    }
}

pub fn render(strokes: &[Vec<Point>], options: &ImageOptions) -> Result<Vec<u8>, String> {
    let mut pixmap = Pixmap::new(options.width, options.height).ok_or("Invalid image size")?;
    if !options.transparent || !options.png {
        pixmap.fill(Color::WHITE);
    }

    let place = placement(strokes, options);
    let [r, g, b] = options.color;
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, 255);
    paint.anti_alias = true;

    let stroke = Stroke {
        width: options.pen_width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };

    for points in strokes {
        let mapped: Vec<(f32, f32)> = points.iter().map(|p| place(*p)).collect();

        if mapped.len() == 1 {
            let (x, y) = mapped[0];
            if let Some(dot) = PathBuilder::from_circle(x, y, options.pen_width / 2.0) {
                pixmap.fill_path(&dot, &paint, FillRule::Winding, Transform::identity(), None);
            }
            continue;
        }

        let mut path = PathBuilder::new();
        for (i, (x, y)) in mapped.iter().enumerate() {
            if i == 0 {
                path.move_to(*x, *y);
            } else {
                path.line_to(*x, *y);
            }
        }
        if let Some(path) = path.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    if options.png {
        return pixmap.encode_png().map_err(|e| e.to_string());
    }

    let rgb: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| [p.red(), p.green(), p.blue()])
        .collect();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode(&rgb, options.width, options.height, ExtendedColorType::Rgb8)
        .map_err(|e| e.to_string())?;
    Ok(jpeg)
}

/// Maps pad coordinates into the image, keeping the aspect ratio and centering the result.
/// By default the whole pad area fills the image, like Topaz's SDK; `imageScaling` zooms to the ink.
fn placement(strokes: &[Vec<Point>], options: &ImageOptions) -> impl Fn(Point) -> (f32, f32) {
    let points = strokes.iter().flatten();
    let (mut left, mut top, mut right, mut bottom) = (X_MIN, Y_MIN, X_MAX, Y_MAX);

    if options.fit_to_signature && strokes.iter().any(|s| !s.is_empty()) {
        left = points.clone().map(|p| p.x).fold(f32::MAX, f32::min);
        right = points.clone().map(|p| p.x).fold(f32::MIN, f32::max);
        top = points.clone().map(|p| p.y).fold(f32::MAX, f32::min);
        bottom = points.map(|p| p.y).fold(f32::MIN, f32::max);
    }

    let margin = options.pen_width * 2.0;
    let (width, height) = (options.width as f32 - margin * 2.0, options.height as f32 - margin * 2.0);
    let (source_width, source_height) = ((right - left).max(1.0), (bottom - top).max(1.0));
    let scale = (width / source_width).min(height / source_height);
    let offset_x = margin + (width - source_width * scale) / 2.0;
    let offset_y = margin + (height - source_height * scale) / 2.0;

    move |p: Point| (offset_x + (p.x - left) * scale, offset_y + (p.y - top) * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_options_from_the_topaz_demo_request() {
        let options = ImageOptions::from_request(&json!({
            "imageFormat": 1, "imageX": 500, "imageY": 100,
            "imageTransparency": false, "imageScaling": false, "penColor": "#ff0000"
        }));

        assert_eq!((options.width, options.height), (500, 100));
        assert!(!options.png);
        assert_eq!(options.color, [255, 0, 0]);
    }

    #[test]
    fn renders_png_and_jpeg() {
        let stroke = vec![Point { x: 500.0, y: 400.0, t: 0 }, Point { x: 2000.0, y: 900.0, t: 9 }];
        let mut options = ImageOptions::from_request(&json!({ "imageFormat": 2 }));

        assert!(render(&[stroke.clone()], &options).unwrap().starts_with(b"\x89PNG"));

        options.png = false;
        assert!(render(&[stroke], &options).unwrap().starts_with(&[0xff, 0xd8]));
    }
}
