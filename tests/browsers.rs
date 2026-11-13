use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

const HOST_NAME: &str = "com.topaz.sigplusextlite.win.2";
const CHROME_ORIGIN: &str = "chrome-extension://dhcpobccjkdnmibckgpejmbpmpembgco/";
const CHROME_ID: &str = "dhcpobccjkdnmibckgpejmbpmpembgco";
const FIREFOX_ID: &str = "sigplusextliteff@topazsystems.com";

fn repo(file: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap()
}

fn fresh_home(test: &str) -> PathBuf {
    let home = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    home
}

fn helperpad(binary: &Path, home: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = loop {
        let spawned = Command::new(binary)
            .args(args)
            .env("HOME", home)
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_STATE_HOME")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        // A test running beside this one can fork while a fresh copy of the binary is still
        // open for writing, and Linux refuses to run it until that fork lets go.
        match spawned {
            Err(e) if e.kind() == ErrorKind::ExecutableFileBusy => thread::sleep(Duration::from_millis(20)),
            spawned => break spawned.unwrap(),
        }
    };
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn run(home: &Path, args: &[&str], input: &[u8]) -> Output {
    helperpad(Path::new(env!("CARGO_BIN_EXE_helperpad")), home, args, input)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn framed(message: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(message).unwrap();
    [&(body.len() as u32).to_ne_bytes()[..], &body].concat()
}

fn unframed(bytes: &[u8]) -> Value {
    let length = u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 4 + length);
    serde_json::from_slice(&bytes[4..]).unwrap()
}

fn asks_for_the_version_as(test: &str, args: &[&str]) -> String {
    let home = fresh_home(test);
    let request = json!({ "metadata": { "command": "GetVersionInfo", "id": 7 } });
    let output = run(&home, args, &framed(&request));

    assert!(output.status.success());
    let response = unframed(&output.stdout);
    assert_eq!(response["status"], 1);
    assert_eq!(response["metadata"], request["metadata"]);

    fs::read_to_string(home.join(".local/state/helperpad/helperpad.log")).unwrap()
}

fn register(home: &Path, dir: &str) -> PathBuf {
    let manifest = home.join(dir).join(format!("{HOST_NAME}.json"));
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, "{}").unwrap();
    manifest
}

#[test]
fn the_chrome_manifest_allows_the_chrome_extension() {
    let manifest: Value = serde_json::from_str(&repo("packaging/com.topaz.sigplusextlite.win.2.json")).unwrap();

    assert_eq!(manifest["name"], HOST_NAME);
    assert_eq!(manifest["type"], "stdio");
    assert_eq!(manifest["path"], "/usr/bin/helperpad");
    assert_eq!(manifest["allowed_origins"], json!([CHROME_ORIGIN]));
    assert!(manifest.get("allowed_extensions").is_none());
}

#[test]
fn the_firefox_manifest_allows_the_firefox_extension() {
    let manifest: Value = serde_json::from_str(&repo("packaging/firefox/com.topaz.sigplusextlite.win.2.json")).unwrap();

    assert_eq!(manifest["name"], HOST_NAME);
    assert_eq!(manifest["type"], "stdio");
    assert_eq!(manifest["path"], "/usr/bin/helperpad");
    assert_eq!(manifest["allowed_extensions"], json!([FIREFOX_ID]));
    assert!(manifest.get("allowed_origins").is_none());
}

#[test]
fn the_installers_register_both_browsers() {
    for script in ["install.sh", "tools/install-from-source.sh"] {
        let script_text = repo(script);

        assert!(script_text.contains("NativeMessagingHosts/$HOST_NAME.json"), "{script} skips Chrome");
        assert!(script_text.contains("packaging/firefox/$HOST_NAME.json"), "{script} skips Firefox");
    }
}

#[test]
fn the_packages_ship_both_manifests() {
    let goreleaser = repo(".goreleaser.yaml");

    for dir in [
        "/etc/opt/chrome/native-messaging-hosts",
        "/etc/chromium/native-messaging-hosts",
        "/usr/lib/mozilla/native-messaging-hosts",
        "/usr/lib64/mozilla/native-messaging-hosts",
    ] {
        assert!(goreleaser.contains(&format!("{dir}/{HOST_NAME}.json")), "no manifest in {dir}");
    }
}

#[test]
fn answers_when_chrome_starts_it() {
    let log = asks_for_the_version_as("chrome-start", &[CHROME_ORIGIN]);

    assert!(log.contains(&format!("started by {CHROME_ORIGIN}")));
}

#[test]
fn answers_when_firefox_starts_it() {
    let manifest = format!("/home/someone/.mozilla/native-messaging-hosts/{HOST_NAME}.json");
    let log = asks_for_the_version_as("firefox-start", &[&manifest, FIREFOX_ID]);

    assert!(log.contains(&format!("started by {FIREFOX_ID}")));
}

#[test]
fn answers_when_started_with_no_arguments() {
    asks_for_the_version_as("bare-start", &[]);
}

#[test]
fn a_mistyped_command_is_an_error_not_a_host() {
    let home = fresh_home("typo");
    let output = run(&home, &["updat"], b"");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(text(&output.stderr).contains("unknown command 'updat'"));
    assert!(!home.join(".local/state/helperpad/helperpad.log").exists());
}

#[test]
fn help_lists_every_command_without_colour_in_a_pipe() {
    let home = fresh_home("help");
    let help = text(&run(&home, &["--help"], b"").stdout);

    for command in ["window", "status", "logs", "update", "update --check", "uninstall", "version", "help"] {
        assert!(help.contains(&format!("  {command} ")), "help misses {command}");
    }
    assert!(help.contains("Chrome and Firefox"));
    assert!(!help.contains('\x1b'));
}

#[test]
fn version_prints_the_cargo_version() {
    let home = fresh_home("version");

    for flag in ["version", "--version", "-V"] {
        let output = run(&home, &[flag], b"");
        assert_eq!(text(&output.stdout).trim(), format!("helperpad {}", env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn status_finds_chrome_and_firefox() {
    let home = fresh_home("status-both");
    let chrome = register(&home, ".config/google-chrome/NativeMessagingHosts");
    let zen = register(&home, ".zen/native-messaging-hosts");
    fs::create_dir_all(home.join(".config/google-chrome/Default/Extensions").join(CHROME_ID)).unwrap();
    let profile = home.join(".zen/abc.default/extensions");
    fs::create_dir_all(&profile).unwrap();
    fs::write(profile.join(format!("{FIREFOX_ID}.xpi")), "").unwrap();

    let status = text(&run(&home, &["status"], b"").stdout);

    assert!(status.contains(&format!("chrome:     registered ({})", chrome.display())), "{status}");
    assert!(status.contains(&format!("firefox:    registered ({})", zen.display())), "{status}");
    assert!(status.contains("installed in Chrome and Firefox"), "{status}");
}

#[test]
fn status_finds_firefox_profiles_two_folders_down() {
    let home = fresh_home("status-firefox");
    let profile = home.join(".mozilla/firefox/abc.default-release/extensions");
    fs::create_dir_all(&profile).unwrap();
    fs::write(profile.join(format!("{FIREFOX_ID}.xpi")), "").unwrap();

    let status = text(&run(&home, &["status"], b"").stdout);

    assert!(status.contains("installed in Firefox"), "{status}");
    assert!(!status.contains("installed in Chrome"), "{status}");
}

#[test]
fn status_says_when_no_extension_is_installed() {
    let home = fresh_home("status-empty");
    let status = text(&run(&home, &["status"], b"").stdout);

    assert!(status.contains("extension:  not found"), "{status}");
}

#[test]
fn uninstall_removes_the_binary_and_both_manifests() {
    let home = fresh_home("uninstall");
    let binary = home.join(".local/bin/helperpad");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_helperpad"), &binary).unwrap();
    let manifests = [
        register(&home, ".config/google-chrome/NativeMessagingHosts"),
        register(&home, ".config/chromium/NativeMessagingHosts"),
        register(&home, ".mozilla/native-messaging-hosts"),
        register(&home, ".zen/native-messaging-hosts"),
    ];

    let output = helperpad(&binary, &home, &["uninstall", "--yes"], b"");

    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(!binary.exists());
    for manifest in manifests {
        assert!(!manifest.exists(), "{} is still there", manifest.display());
    }
}

#[test]
fn uninstall_keeps_everything_without_a_yes() {
    let home = fresh_home("uninstall-no");
    let binary = home.join(".local/bin/helperpad");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_helperpad"), &binary).unwrap();
    let manifest = register(&home, ".mozilla/native-messaging-hosts");

    let output = helperpad(&binary, &home, &["uninstall"], b"n\n");

    assert!(text(&output.stdout).contains("Nothing removed."));
    assert!(binary.exists());
    assert!(manifest.exists());
}
