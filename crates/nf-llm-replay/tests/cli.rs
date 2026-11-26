//! The `nf-llm-replay` binary: flags, env vars, exit codes and the `listening on` line.

// Helpers outside #[test] fns are not covered by clippy.toml's allow-*-in-tests.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;

use std::io::{BufRead, BufReader};
use std::process::{Command, Output, Stdio};

/// The binary with no inherited `NF_LLM_REPLAY_*` variables.
fn bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_nf-llm-replay"));
    for (name, _) in std::env::vars().filter(|(n, _)| n.starts_with("NF_LLM_REPLAY_")) {
        cmd.env_remove(name);
    }
    cmd
}

fn run(cmd: &mut Command) -> (i32, String, String) {
    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().unwrap();
    let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (status.code().unwrap(), text(stdout), text(stderr))
}

#[test]
fn help_lists_flags_and_env_vars() {
    let (code, out, _) = run(bin().arg("--help"));
    assert_eq!(code, 0);
    for flag in ["mode", "cassette", "upstream", "port", "bind"] {
        assert!(out.contains(&format!("--{flag}")), "{flag}: {out}");
        let env = format!("NF_LLM_REPLAY_{}", flag.to_uppercase());
        assert!(out.contains(&env), "{env}: {out}");
    }
    assert!(
        out.contains("NF_LLM_REPLAY_API_KEY") && !out.contains("--api-key"),
        "{out}"
    );
}

#[test]
fn invalid_mode_exits_2() {
    let (code, _, err) = run(bin().args(["--mode", "bogus", "--cassette", "c.jsonl"]));
    assert_eq!(code, 2);
    assert!(err.contains("bogus"), "{err}");
    let (code, _, err) = run(bin().args(["--mode", "replay"]));
    assert_eq!(code, 2, "replay without --cassette is a usage error");
    assert!(err.contains("--cassette"), "{err}");
}

#[test]
fn mode_from_env_var_is_honoured() {
    let (code, _, err) = run(bin()
        .env("NF_LLM_REPLAY_MODE", "bogus")
        .args(["--cassette", "c.jsonl"]));
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("bogus"), "{err}");

    let dir = tempfile::tempdir().unwrap();
    let cassette = common::write_cassette(dir.path(), &[]);
    let mut child = bin()
        .env("NF_LLM_REPLAY_MODE", "replay")
        .env("NF_LLM_REPLAY_CASSETTE", &cassette)
        .env("NF_LLM_REPLAY_PORT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(line.starts_with("listening on 127.0.0.1:"), "{line}");
}

#[test]
fn replay_with_missing_cassette_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.jsonl");
    let (code, out, err) = run(bin().arg("--cassette").arg(&missing).args(["--port", "0"]));
    assert_eq!(code, 1);
    assert!(!out.contains("listening"), "{out}");
    assert!(err.contains(missing.to_str().unwrap()), "{err}");
    assert_eq!(err.trim().lines().count(), 1, "{err}");
}

#[test]
fn corrupt_cassette_exits_1_with_line_number() {
    let dir = tempfile::tempdir().unwrap();
    let good = common::entry("GET", "/v1/models", "", 200, None, "ok");
    let cassette = common::write_cassette(dir.path(), &[good, "{not json".into()]);
    let (code, _, err) = run(bin().arg("--cassette").arg(&cassette).args(["--port", "0"]));
    assert_eq!(code, 1);
    assert!(err.contains("line 2"), "{err}");
    assert_eq!(err.trim().lines().count(), 1, "{err}");
}

#[tokio::test]
async fn port_0_prints_listening_address_and_serves() {
    let dir = tempfile::tempdir().unwrap();
    let cassette = common::write_cassette(
        dir.path(),
        &[common::entry("GET", "/v1/models", "", 200, None, "ok")],
    );
    let mut child = bin()
        .arg("--cassette")
        .arg(&cassette)
        .args(["--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let addr = line.trim().strip_prefix("listening on ").unwrap();
    assert!(
        addr.starts_with("127.0.0.1:") && !addr.ends_with(":0"),
        "{line}"
    );

    let reply = common::send_raw(
        addr.parse().unwrap(),
        b"GET /v1/models HTTP/1.1\r\nHost: t\r\n\r\n",
    )
    .await;
    assert_eq!((reply.status, reply.text()), (200, "ok".to_owned()));

    // SIGTERM shuts down gracefully (exit 0); the request was logged to stderr.
    let pid = child.id().to_string();
    assert!(
        Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.status);
    let log = String::from_utf8(output.stderr).unwrap();
    assert!(log.contains("/v1/models") && log.contains("hit"), "{log}");
}

#[test]
fn record_without_api_key_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("rec.jsonl");
    for key in [None, Some("")] {
        let mut cmd = bin();
        cmd.args([
            "--mode",
            "record",
            "--upstream",
            "http://127.0.0.1:9",
            "--port",
            "0",
        ])
        .arg("--cassette")
        .arg(&cassette);
        if let Some(key) = key {
            cmd.env("NF_LLM_REPLAY_API_KEY", key);
        }
        let (code, out, err) = run(&mut cmd);
        assert_eq!(code, 1, "{key:?}: {err}");
        assert!(!out.contains("listening"), "{out}");
        assert!(err.contains("NF_LLM_REPLAY_API_KEY"), "{err}");
        assert_eq!(err.trim().lines().count(), 1, "{err}");
        assert!(!cassette.exists(), "no cassette without a key");
    }
}

#[test]
fn record_without_upstream_exits_2() {
    for mode in ["record", "passthrough"] {
        let (code, _, err) = run(bin().env("NF_LLM_REPLAY_API_KEY", "k").args([
            "--mode",
            mode,
            "--cassette",
            "c.jsonl",
        ]));
        assert_eq!(code, 2, "{mode}: {err}");
        assert!(err.contains("--upstream"), "{err}");
    }
}

#[tokio::test]
async fn api_key_never_appears_in_output() {
    let (up, fake) = common::fake_upstream::spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let mut child = bin()
        .env("NF_LLM_REPLAY_API_KEY", common::KEY)
        .env("RUST_LOG", "trace")
        .args(["--mode", "record", "--port", "0", "--upstream", &up])
        .arg("--cassette")
        .arg(dir.path().join("rec.jsonl"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let addr = line
        .trim()
        .strip_prefix("listening on ")
        .unwrap()
        .parse()
        .unwrap();

    let body = br#"{"model":"m","messages":[{"role":"user","content":"q"}]}"#;
    let reply = common::post_with(addr, "/v1/chat/completions", "x-api-key: dummy\r\n", body).await;
    assert_eq!(reply.status, 200);
    assert_eq!(fake.hits.load(std::sync::atomic::Ordering::SeqCst), 1);

    let pid = child.id().to_string();
    assert!(
        Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .unwrap()
            .success()
    );
    let output = child.wait_with_output().unwrap();
    let mut all = line;
    std::io::Read::read_to_string(&mut stdout, &mut all).unwrap();
    all += &String::from_utf8(output.stderr).unwrap();
    assert!(all.contains("TRACE") && all.contains("recorded"), "{all}");
    assert!(!all.contains(common::KEY), "key leaked:\n{all}");
}
