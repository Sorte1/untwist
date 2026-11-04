#![allow(dead_code)]

use std::io::Write;
use std::process::{Command, Stdio};

pub fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_untwist"))
}

pub fn stdin_output(source: &str, args: &[&str]) -> std::process::Output {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let written = child.stdin.take().unwrap().write_all(source.as_bytes());
    if let Err(error) = written {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe, "{error}");
    }
    child.wait_with_output().unwrap()
}

#[path = "../../../untwist/tests/common/mod.rs"]
mod library_common;
#[allow(unused_imports)]
pub use library_common::WEBPACK_4;

pub const SPLIT_STRINGS_BEHIND_A_DECODER: &str = "\
function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
console.log(b(0x0) + b(0x1));";

pub const A_DECODER_THAT_SPENDS_OVER_50M_STEPS: &str = "\
function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var s = 0; for (var i = 0; i < 180; i++) { for (var j = 0; j < 180; j++) { for (var k = 0; k < 180; k++) { s = s + 1; } } } return e[f]; }; return b(c, d); }
console.log(b(0x0) + b(0x1));";
