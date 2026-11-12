use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::update::{self, Install, Package};
use crate::{log, pad};

const HOST_NAME: &str = "com.topaz.sigplusextlite.win.2";
const EXTENSION_ID: &str = "dhcpobccjkdnmibckgpejmbpmpembgco";
const STORE_URL: &str = "https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/dhcpobccjkdnmibckgpejmbpmpembgco";
const UDEV_RULES: [&str; 2] = ["/etc/udev/rules.d/70-topaz-sigpad.rules", "/usr/lib/udev/rules.d/70-topaz-sigpad.rules"];
const SYSTEM_MANIFEST_DIRS: [&str; 2] = ["/etc/opt/chrome/native-messaging-hosts", "/etc/chromium/native-messaging-hosts"];

pub fn status() -> ExitCode {
    // Lets `helperpad status | head` end quietly instead of failing on the closed pipe.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };

    let row = |label: &str, value: String| println!("{:<11} {value}", format!("{label}:"));

    row("version", update::CURRENT.to_string());
    row("binary", crate::own_path().map_or_else(|e| e, |path| path.display().to_string()));

    let manifests = manifests();
    if manifests.is_empty() {
        row("chrome", "not registered (install helperpad again)".to_string());
    }
    for manifest in manifests {
        row("chrome", format!("registered ({})", manifest.display()));
    }

    row("extension", match extension_installed() {
        true => "installed".to_string(),
        false => format!("not found in Chrome ({STORE_URL})"),
    });

    row("pad", match pad::find() {
        None => "not plugged in".to_string(),
        Some(device) if File::open(&device).is_ok() => format!("{} (readable)", device.display()),
        Some(device) => format!("{} (no access: replug the pad)", device.display()),
    });

    row("udev rule", match UDEV_RULES.iter().find(|rule| Path::new(rule).exists()) {
        Some(rule) => rule.to_string(),
        None => "missing".to_string(),
    });

    let log = log::path();
    let lines = File::open(&log).map(|file| io::BufReader::new(file).lines().count());
    row("log", match lines {
        Ok(lines) => format!("{} ({lines} lines)", log.display()),
        Err(_) => format!("{} (empty)", log.display()),
    });

    ExitCode::SUCCESS
}

pub fn logs() -> ExitCode {
    let log = log::path();
    if let Some(dir) = log.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::OpenOptions::new().create(true).append(true).open(&log);

    let error = Command::new("tail").args(["-n", "50", "-f"]).arg(&log).exec();
    eprintln!("helperpad: couldn't run tail on {}: {error}", log.display());
    ExitCode::FAILURE
}

pub fn uninstall(args: &[String]) -> ExitCode {
    let confirmed = args.iter().any(|arg| arg == "--yes" || arg == "-y");
    let binary = match crate::own_path() {
        Ok(binary) => binary,
        Err(e) => {
            eprintln!("helperpad: {e}");
            return ExitCode::FAILURE;
        }
    };

    if !confirmed && !ask(&format!("Remove helperpad ({})? [y/N] ", binary.display())) {
        println!("Nothing removed.");
        return ExitCode::SUCCESS;
    }

    let result = match update::install_kind(&binary) {
        Install::Package(package) => remove_package(package),
        Install::Binary(binary) => remove_files(&binary),
    };

    match result {
        Ok(()) => {
            println!("helperpad is uninstalled. The log stays in {}.", log::path().display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Uninstall failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn remove_package(package: Package) -> Result<(), String> {
    let (program, args): (&str, &[&str]) = match package {
        Package::Deb => ("apt-get", &["remove", "-y"]),
        Package::Arch => ("pacman", &["-R", "--noconfirm"]),
        Package::Rpm if update::on_path("dnf") => ("dnf", &["remove", "-y"]),
        Package::Rpm if update::on_path("zypper") => ("zypper", &["--non-interactive", "remove"]),
        Package::Rpm => ("rpm", &["-e"]),
    };

    println!("Removing the helperpad package (asks for your password)");
    update::run_as_root(program, args, Path::new("helperpad"))
}

fn remove_files(binary: &Path) -> Result<(), String> {
    for manifest in user_manifests() {
        fs::remove_file(&manifest).map_err(|e| format!("can't remove {}: {e}", manifest.display()))?;
        println!("Removed {}", manifest.display());
    }
    fs::remove_file(binary).map_err(|e| format!("can't remove {}: {e}", binary.display()))?;
    println!("Removed {}", binary.display());

    if Path::new(UDEV_RULES[0]).exists() {
        println!("The pad's udev rule is still there. To remove it too: sudo rm {}", UDEV_RULES[0]);
    }
    Ok(())
}

fn ask(question: &str) -> bool {
    print!("{question}");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    let _ = io::stdin().read_line(&mut answer);

    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

fn user_manifests() -> Vec<PathBuf> {
    let Some(config) = config_dir() else {
        return Vec::new();
    };

    ["google-chrome", "chromium"]
        .iter()
        .map(|browser| config.join(browser).join("NativeMessagingHosts").join(format!("{HOST_NAME}.json")))
        .filter(|manifest| manifest.exists())
        .collect()
}

fn manifests() -> Vec<PathBuf> {
    let system = SYSTEM_MANIFEST_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(format!("{HOST_NAME}.json")))
        .filter(|manifest| manifest.exists());

    user_manifests().into_iter().chain(system).collect()
}

/// Chrome keeps extensions under `<browser>/<profile>/Extensions/<id>`.
fn extension_installed() -> bool {
    let Some(config) = config_dir() else {
        return false;
    };
    let subdirs = |dir: PathBuf| fs::read_dir(dir).into_iter().flatten().flatten().map(|entry| entry.path());

    subdirs(config)
        .flat_map(subdirs)
        .any(|profile| profile.join("Extensions").join(EXTENSION_ID).is_dir())
}
