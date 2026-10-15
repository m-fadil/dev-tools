#![allow(dead_code)]
use std::io::Write;
use std::process::{Command, Stdio};

pub struct Out {
    pub code: i32,
    pub out: String,
    pub err: String,
    pub out_bytes: Vec<u8>,
}

/// Runs `dt` with a cleared environment (PATH only), stdin piped and never a terminal.
pub fn dt_env(args: &[&str], stdin: &[u8], env: &[(&str, &str)]) -> Out {
    dt_in_dir(args, stdin, env, None)
}

pub fn dt_in_dir(args: &[&str], stdin: &[u8], env: &[(&str, &str)], cwd: Option<&std::path::Path>) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dt"));
    cmd.args(args).env_clear();
    for key in ["PATH", "SystemRoot"] {
        if let Some(v) = std::env::var_os(key) {
            cmd.env(key, v);
        }
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    let data = stdin.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = pipe.write_all(&data);
    });
    let output = child.wait_with_output().unwrap();
    let _ = writer.join();
    Out {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
        out_bytes: output.stdout,
    }
}

pub fn dt_in(args: &[&str], stdin: &[u8]) -> Out {
    dt_env(args, stdin, &[])
}

pub fn dt(args: &[&str]) -> Out {
    dt_in(args, b"")
}
