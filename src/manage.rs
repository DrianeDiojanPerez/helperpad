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
const FIREFOX_EXTENSION_ID: &str = "sigplusextliteff@topazsystems.com";
const FIREFOX_URL: &str = "https://addons.mozilla.org/firefox/addon/topaz-sigplusextlite-extension/";
const FIREFOX_HOMES: [&str; 5] = [".mozilla", ".zen", ".librewolf", ".floorp", ".waterfox"];
const UDEV_RULES: [&str; 2] = ["/etc/udev/rules.d/70-topaz-sigpad.rules", "/usr/lib/udev/rules.d/70-topaz-sigpad.rules"];
const SYSTEM_MANIFEST_DIRS: [&str; 4] = [
    "/etc/opt/chrome/native-messaging-hosts",
    "/etc/chromium/native-messaging-hosts",
    "/usr/lib/mozilla/native-messaging-hosts",
    "/usr/lib64/mozilla/native-messaging-hosts",
];

pub fn status() -> ExitCode {
    // Lets `helperpad status | head` end quietly instead of failing on the closed pipe.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };

    let row = |label: &str, value: String| println!("{:<11} {value}", format!("{label}:"));

    row("version", update::CURRENT.to_string());
    row("binary", crate::own_path().map_or_else(|e| e, |path| path.display().to_string()));

    let manifests = manifests();
    if manifests.is_empty() {
        row("browser", "not registered (install helperpad again)".to_string());
    }
    for manifest in manifests {
        let browser = if is_firefox_manifest(&manifest) { "firefox" } else { "chrome" };
        row(browser, format!("registered ({})", manifest.display()));
    }

    match (chrome_extension_installed(), firefox_extension_installed()) {
        (false, false) => {
            row("extension", format!("not found. Chrome: {STORE_URL}"));
            println!("{:<11} Firefox: {FIREFOX_URL}", "");
        }
        (chrome, firefox) => {
            let browsers: Vec<&str> = [(chrome, "Chrome"), (firefox, "Firefox")]
                .into_iter()
                .filter_map(|(installed, browser)| installed.then_some(browser))
                .collect();
            row("extension", format!("installed in {}", browsers.join(" and ")));
        }
    }

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

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn is_firefox_manifest(manifest: &Path) -> bool {
    manifest.parent().is_some_and(|dir| dir.ends_with("native-messaging-hosts") && !dir.starts_with("/etc"))
}

fn user_manifests() -> Vec<PathBuf> {
    let file = format!("{HOST_NAME}.json");
    let chrome = config_dir().into_iter().flat_map(|config| {
        ["google-chrome", "chromium"].map(|browser| config.join(browser).join("NativeMessagingHosts"))
    });
    let firefox = home()
        .into_iter()
        .flat_map(|home| FIREFOX_HOMES.map(|dir| home.join(dir).join("native-messaging-hosts")));

    chrome.chain(firefox).map(|dir| dir.join(&file)).filter(|manifest| manifest.exists()).collect()
}

fn manifests() -> Vec<PathBuf> {
    let system = SYSTEM_MANIFEST_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(format!("{HOST_NAME}.json")))
        .filter(|manifest| manifest.exists());

    // On Arch /usr/lib64 is a link to /usr/lib, so the same file would show up twice.
    let mut seen = Vec::new();
    user_manifests()
        .into_iter()
        .chain(system)
        .filter(|manifest| {
            let real = fs::canonicalize(manifest).unwrap_or_else(|_| manifest.clone());
            let new = !seen.contains(&real);
            seen.push(real);
            new
        })
        .collect()
}

fn subdirs(dir: PathBuf) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(dir).into_iter().flatten().flatten().map(|entry| entry.path())
}

/// Chrome keeps extensions under `<browser>/<profile>/Extensions/<id>`.
fn chrome_extension_installed() -> bool {
    config_dir().is_some_and(|config| {
        subdirs(config)
            .flat_map(subdirs)
            .any(|profile| profile.join("Extensions").join(EXTENSION_ID).is_dir())
    })
}

/// Firefox keeps them under `<profile>/extensions/<id>.xpi`, with profiles one or two folders down.
fn firefox_extension_installed() -> bool {
    let has_extension = |profile: &PathBuf| profile.join("extensions").join(format!("{FIREFOX_EXTENSION_ID}.xpi")).exists();

    home().is_some_and(|home| {
        FIREFOX_HOMES.iter().any(|dir| {
            let folders: Vec<PathBuf> = subdirs(home.join(dir)).collect();
            folders.iter().any(has_extension) || folders.into_iter().flat_map(subdirs).any(|profile| has_extension(&profile))
        })
    })
}
