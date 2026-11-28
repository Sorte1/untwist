#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../harness/options.rs"]
mod harness;

fuzz_target!(|data: &[u8]| harness::run(data));
