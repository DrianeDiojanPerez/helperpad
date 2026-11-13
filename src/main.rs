mod host;
mod log;
mod manage;
mod pad;
mod rawdata;
mod render;
mod sigcrypt;
mod sigstring;
mod update;
mod window;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("window" | "sign" | "--sign") => sign(&args[2..]),
        Some("update") => update::run(&args[2..]),
        Some("status") => manage::status(),
        Some("logs") => manage::logs(),
        Some("uninstall") => manage::uninstall(&args[2..]),
        Some("version" | "--version" | "-V") => {
            println!("helperpad {}", update::CURRENT);
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        // Chrome passes the calling extension as the first argument and Firefox the path
        // of its registration file; anything else is a typo.
        Some(command) if !command.starts_with("chrome-extension://") && !command.ends_with(".json") => {
            eprintln!("helperpad: unknown command '{command}'");
            eprintln!("Run 'helperpad help' to see the commands.");
            ExitCode::from(2)
        }
        None if unsafe { libc::isatty(0) } == 1 => {
            print_help();
            ExitCode::SUCCESS
        }
        _ => {
            let caller = args.get(2).or(args.get(1)).map_or("unknown", String::as_str);
            log::write(&format!("started by {caller}"));
            match host::serve() {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    log::write(&format!("stopped: {e}"));
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn print_help() {
    let colour = unsafe { libc::isatty(1) } == 1 && env::var_os("NO_COLOR").is_none();
    let paint = |code: &str, text: &str| match colour {
        true => format!("\x1b[{code}m{text}\x1b[0m"),
        false => text.to_string(),
    };
    let heading = |text: &str| paint("1;32", text);
    let name = |text: &str| paint("1;36", text);

    println!("{} {}", paint("1", "helperpad"), update::CURRENT);
    println!("Topaz signature pads in Chrome and Firefox on Linux");
    println!();
    println!("{} {} {}", heading("Usage:"), name("helperpad"), paint("36", "<COMMAND>"));
    println!();
    println!("{}", heading("Commands:"));
    println!("  {}           Open the signing window to test the pad", name("window"));
    println!("  {}           Show what's installed and whether the pad is found", name("status"));
    println!("  {}             Watch the log live (Ctrl+C to stop)", name("logs"));
    println!("  {}           Download and install the newest version", name("update"));
    println!("  {}   Only say whether a newer version exists", name("update --check"));
    println!("  {}        Remove helperpad (add --yes to skip the question)", name("uninstall"));
    println!("  {}          Print the version", name("version"));
    println!("  {}             Print this help", name("help"));
    println!();
    println!("Chrome and Firefox start helperpad on their own when a website asks for a signature.");
    println!("{} {}", paint("2", "Log:"), log::path().display());
}

/// After a reinstall, `current_exe` names the deleted old binary while the path Chrome launched still works.
pub fn own_path() -> Result<PathBuf, String> {
    env::args_os()
        .next()
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.exists())
        .map_or_else(env::current_exe, Ok)
        .map_err(|e| e.to_string())
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
