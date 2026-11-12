mod pad;
mod render;
mod sigstring;
mod window;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("--sign") => sign(&args[2..]),
        Some("--help" | "-h") => {
            println!("helperpad: Topaz SigPlusExtLite native messaging host for Linux.");
            println!("Chrome starts it on its own. Run `helperpad --sign` to test the signing window.");
            ExitCode::SUCCESS
        }
        _ => {
            println!("Run `helperpad --sign` to open the signing window.");
            ExitCode::SUCCESS
        }
    }
}

/// Child mode: shows the signing window and prints the strokes as JSON on stdout.
fn sign(args: &[String]) -> ExitCode {
    let device = args.first().map(PathBuf::from).or_else(pad::find);
    let min_points = args.get(1).and_then(|n| n.parse().ok()).unwrap_or(25);

    let result = match device {
        None => json!({ "error": "No Topaz signature pad found." }),
        Some(device) => match window::run(device, min_points) {
            Ok(window::Outcome::Accepted(strokes)) => json!({
                "strokes": strokes
                    .iter()
                    .map(|s| s.iter().map(|p| [p.x, p.y, p.t as f32]).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
            }),
            Ok(window::Outcome::Cancelled) => json!({ "cancelled": true }),
            Err(e) => json!({ "error": e }),
        },
    };

    println!("{result}");
    ExitCode::SUCCESS
}
