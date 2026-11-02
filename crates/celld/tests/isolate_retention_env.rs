// Copyright 2026 Deno Land Inc. Apache-2.0 license.

use std::process::{Command, Output};

fn startup(value: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_celld"));
    command.env_clear().arg("--version");
    if let Some(value) = value {
        command.env("CELLD_KEEP_ONE_ISOLATE", value);
    }
    command.output().expect("run celld startup validation")
}

#[test]
fn keep_one_isolate_accepts_unset_zero_and_one() {
    for value in [None, Some("0"), Some("1")] {
        let output = startup(value);
        assert!(
            output.status.success(),
            "CELLD_KEEP_ONE_ISOLATE={value:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn keep_one_isolate_rejects_invalid_values_at_startup() {
    for value in ["", "true", "false", "2", "-1", " 1", "1 "] {
        let output = startup(Some(value));
        assert!(
            !output.status.success(),
            "accepted CELLD_KEEP_ONE_ISOLATE={value:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("CELLD_KEEP_ONE_ISOLATE must be 0 or 1"),
            "unexpected error for {value:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
