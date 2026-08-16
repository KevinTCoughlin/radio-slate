use std::process::Command;

use tempfile::TempDir;

fn run_cli(args: &[&str]) -> std::process::Output {
    let config_home = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_radio-slate"))
        .args(args)
        .env("XDG_CONFIG_HOME", config_home.path())
        .output()
        .unwrap()
}

#[test]
fn json_list_output_is_one_valid_document() {
    let output = run_cli(&["--list", "--format", "json"]);

    assert!(output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["ready"], true);
    assert_eq!(payload["list_requested"], true);
    assert_eq!(payload["stations"].as_array().unwrap().len(), 1);
}

#[test]
fn json_status_reports_the_service_state() {
    let output = run_cli(&["--status", "--format", "json"]);

    assert!(output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["playback_state"], "stopped");
    assert_eq!(payload["status"], "stopped");
}
