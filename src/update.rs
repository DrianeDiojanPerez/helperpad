use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use crate::log;

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

const REPO: &str = "DrianeDiojanPerez/helperpad";
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_DOWNLOAD: u64 = 200 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Package {
    Deb,
    Rpm,
    Arch,
}

#[derive(Debug, PartialEq)]
pub enum Install {
    Binary(PathBuf),
    Package(Package),
}

pub fn run(args: &[String]) -> ExitCode {
    let check_only = args.first().is_some_and(|arg| arg == "--check");

    let latest = match latest_version(Duration::from_secs(30)) {
        Ok(latest) => latest,
        Err(e) => {
            eprintln!("Couldn't check for updates: {e}");
            return ExitCode::FAILURE;
        }
    };
    remember(&latest);

    if !is_newer(&latest, CURRENT) {
        println!("helperpad {CURRENT} is the newest version.");
        return ExitCode::SUCCESS;
    }

    if check_only {
        println!("helperpad {latest} is available (you have {CURRENT}). Run `helperpad update` to install it.");
        return ExitCode::SUCCESS;
    }

    println!("Updating helperpad {CURRENT} to {latest}");
    match install(&latest, &|step| println!("  {step}")) {
        Ok(()) => {
            println!("Updated to {latest}. Close Chrome completely and open it again to finish.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Update failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The newer version, if there is one. Asks GitHub at most once a day and never fails loudly,
/// because the signing window calls it on every signature.
pub fn available() -> Option<String> {
    let latest = match remembered() {
        Some(latest) => latest,
        None => {
            let latest = latest_version(Duration::from_secs(5)).ok()?;
            remember(&latest);
            latest
        }
    };

    is_newer(&latest, CURRENT).then_some(latest)
}

pub fn install(version: &str, report: &dyn Fn(&str)) -> Result<(), String> {
    let result = download_and_install(version, report);
    match &result {
        Ok(()) => log::write(&format!("updated from {CURRENT} to {version}")),
        Err(e) => log::write(&format!("update to {version} failed: {e}")),
    }
    result
}

fn download_and_install(version: &str, report: &dyn Fn(&str)) -> Result<(), String> {
    let target = crate::own_path()?;
    let install = install_kind(&target);
    let name = asset_name(&install, version, std::env::consts::ARCH)
        .ok_or_else(|| format!("there are no builds for {}", std::env::consts::ARCH))?;
    let base = format!("https://github.com/{REPO}/releases/download/v{version}");
    let timeout = Duration::from_secs(300);

    report(&format!("Downloading {name}"));
    let file = fetch(&format!("{base}/{name}"), timeout)?;
    let checksums = String::from_utf8(fetch(&format!("{base}/checksums.txt"), timeout)?)
        .map_err(|_| "the checksum list isn't text".to_string())?;

    report("Checking the download");
    let expected = checksum_for(&checksums, &name).ok_or("the release has no checksum for this file")?;
    if hex(&Sha256::digest(&file)) != expected.to_lowercase() {
        return Err("the download doesn't match its checksum".to_string());
    }

    match install {
        Install::Binary(path) => {
            report(&format!("Replacing {}", path.display()));
            replace_binary(&file, &path)
        }
        Install::Package(package) => {
            report("Installing the package (asks for your password)");
            install_package(package, &name, &file)
        }
    }
}

fn latest_version(timeout: Duration) -> Result<String, String> {
    let body = fetch(&format!("https://api.github.com/repos/{REPO}/releases/latest"), timeout)?;
    let release: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    let tag = release["tag_name"].as_str().ok_or("GitHub didn't name a release")?;
    let version = tag.trim_start_matches('v');

    // The version ends up in download URLs and file names, so only plain numbers get through.
    parse_version(version).ok_or_else(|| format!("unexpected release name {tag}"))?;
    Ok(version.to_string())
}

fn fetch(url: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(timeout)).build().into();

    agent
        .get(url)
        .header("User-Agent", concat!("helperpad/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

fn parse_version(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.').map(|part| part.parse::<u64>().ok());
    let parsed = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(parsed)
}

fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

fn asset_name(install: &Install, version: &str, arch: &str) -> Option<String> {
    let (archive_arch, package_arch) = match arch {
        "x86_64" => ("x86_64", "amd64"),
        "aarch64" => ("arm64", "arm64"),
        _ => return None,
    };

    Some(match install {
        Install::Binary(_) => format!("helperpad_Linux_{archive_arch}.tar.gz"),
        Install::Package(Package::Deb) => format!("helperpad_{version}_linux_{package_arch}.deb"),
        Install::Package(Package::Rpm) => format!("helperpad_{version}_linux_{package_arch}.rpm"),
        Install::Package(Package::Arch) => format!("helperpad_{version}_linux_{package_arch}.pkg.tar.zst"),
    })
}

fn checksum_for<'a>(checksums: &'a str, name: &str) -> Option<&'a str> {
    checksums.lines().find_map(|line| {
        let (sum, file) = line.split_once(char::is_whitespace)?;
        (file.trim() == name).then_some(sum)
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A binary a package manager owns has to be updated through it, or the next package
/// upgrade would trip over files it doesn't recognise.
pub fn install_kind(target: &Path) -> Install {
    let owns = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .arg(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };

    if owns("dpkg", &["-S"]) {
        Install::Package(Package::Deb)
    } else if owns("rpm", &["-qf"]) {
        Install::Package(Package::Rpm)
    } else if owns("pacman", &["-Qqo"]) {
        Install::Package(Package::Arch)
    } else {
        Install::Binary(target.to_path_buf())
    }
}

fn replace_binary(archive: &[u8], target: &Path) -> Result<(), String> {
    let mut archive = tar::Archive::new(GzDecoder::new(archive));
    let mut binary = Vec::new();

    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        if entry.path().map_err(|e| e.to_string())?.as_ref() == Path::new("helperpad") {
            entry.read_to_end(&mut binary).map_err(|e| e.to_string())?;
            break;
        }
    }
    if binary.is_empty() {
        return Err("the download has no helperpad binary in it".to_string());
    }

    // Renaming over the old file keeps a running helper working until Chrome restarts it.
    let staged = target.with_file_name(".helperpad.update");
    let describe = |e: io::Error| format!("can't write {}: {e}", target.display());
    fs::write(&staged, binary).map_err(describe)?;
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o755)).map_err(describe)?;
    fs::rename(&staged, target).map_err(describe)
}

fn install_package(package: Package, name: &str, file: &[u8]) -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("helperpad-update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::DirBuilder::new().mode(0o755).create(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(name);

    let result = fs::write(&path, file).map_err(|e| e.to_string()).and_then(|()| {
        let (program, args): (&str, &[&str]) = match package {
            Package::Deb => ("apt-get", &["install", "-y"]),
            Package::Arch => ("pacman", &["-U", "--noconfirm"]),
            Package::Rpm if on_path("dnf") => ("dnf", &["install", "-y"]),
            Package::Rpm if on_path("zypper") => ("zypper", &["--non-interactive", "install", "--allow-unsigned-rpm"]),
            Package::Rpm => ("rpm", &["-U"]),
        };
        run_as_root(program, args, &path)
    });

    let _ = fs::remove_dir_all(&dir);
    result
}

pub fn run_as_root(program: &str, args: &[&str], target: &Path) -> Result<(), String> {
    let in_terminal = unsafe { libc::isatty(0) } == 1;
    let mut command = if unsafe { libc::geteuid() } == 0 {
        Command::new(program)
    } else {
        let elevate = if in_terminal && on_path("sudo") {
            "sudo"
        } else if on_path("pkexec") {
            "pkexec"
        } else {
            return Err("this needs root; run the command in a terminal".to_string());
        };
        let mut command = Command::new(elevate);
        command.arg(program);
        command
    };

    // The signing window's stdout carries the signature to the host, so tool output goes to stderr.
    let status = command
        .args(args)
        .arg(target)
        .stdin(if in_terminal { Stdio::inherit() } else { Stdio::null() })
        .stdout(io::stderr())
        .status()
        .map_err(|e| format!("couldn't start {program}: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} stopped with {status}; run the command in a terminal to see why"))
    }
}

pub fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

fn check_file() -> PathBuf {
    log::path().with_file_name("update-check")
}

fn remembered() -> Option<String> {
    let saved = fs::read_to_string(check_file()).ok()?;
    let (checked_at, version) = saved.trim().split_once(' ')?;
    let checked_at = UNIX_EPOCH + Duration::from_secs(checked_at.parse().ok()?);
    let age = SystemTime::now().duration_since(checked_at).ok()?;

    (age < CHECK_EVERY && parse_version(version).is_some()).then(|| version.to_string())
}

fn remember(version: &str) {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return;
    };
    let file = check_file();
    if let Some(dir) = file.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(file, format!("{} {version}\n", now.as_secs()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn refuses_versions_that_are_not_three_numbers() {
        assert_eq!(parse_version("1.2.3"), Some([1, 2, 3]));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("1.2.3-rc1"), None);
        assert_eq!(parse_version("../../etc"), None);
        assert!(!is_newer("9.9.9-rc1", "0.1.0"));
    }

    #[test]
    fn names_the_files_goreleaser_publishes() {
        let binary = Install::Binary(PathBuf::from("/home/me/.local/bin/helperpad"));

        assert_eq!(asset_name(&binary, "0.1.0", "x86_64").unwrap(), "helperpad_Linux_x86_64.tar.gz");
        assert_eq!(asset_name(&binary, "0.1.0", "aarch64").unwrap(), "helperpad_Linux_arm64.tar.gz");
        assert_eq!(
            asset_name(&Install::Package(Package::Deb), "0.1.0", "x86_64").unwrap(),
            "helperpad_0.1.0_linux_amd64.deb"
        );
        assert_eq!(
            asset_name(&Install::Package(Package::Rpm), "0.1.0", "aarch64").unwrap(),
            "helperpad_0.1.0_linux_arm64.rpm"
        );
        assert_eq!(
            asset_name(&Install::Package(Package::Arch), "0.1.0", "x86_64").unwrap(),
            "helperpad_0.1.0_linux_amd64.pkg.tar.zst"
        );
        assert_eq!(asset_name(&binary, "0.1.0", "riscv64"), None);
    }

    #[test]
    fn finds_a_file_in_the_checksum_list() {
        let checksums = "00868a8a  helperpad_0.1.0_linux_amd64.deb\n4625669a  helperpad_Linux_x86_64.tar.gz\n";

        assert_eq!(checksum_for(checksums, "helperpad_Linux_x86_64.tar.gz"), Some("4625669a"));
        assert_eq!(checksum_for(checksums, "helperpad_Linux_arm64.tar.gz"), None);
    }

    #[test]
    fn swaps_in_the_binary_from_a_release_archive() {
        let dir = std::env::temp_dir().join(format!("helperpad-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("helperpad");
        fs::write(&target, "old").unwrap();

        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast()));
        for (name, body) in [("README.md", "docs"), ("helperpad", "new binary")] {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, body.as_bytes()).unwrap();
        }
        let archive = builder.into_inner().unwrap().finish().unwrap();

        replace_binary(&archive, &target).unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "new binary");
        assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755);
        assert!(replace_binary(&[], &target).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
