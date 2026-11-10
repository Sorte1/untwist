use std::time::Duration;

use untwist_core::budget::{self, Deadline, Limits};
use untwist_unpack::wasm::may_embed_wasm;

const UNIT: usize = 15;
const NESTED: usize = 32_000;
const BOUND: Duration = Duration::from_secs(2);

fn leb5(mut value: usize) -> [u8; 5] {
    let mut out = [0u8; 5];
    for (index, byte) in out.iter_mut().enumerate() {
        let more = if index < 4 { 0x80 } else { 0 };
        *byte = (value & 0x7f) as u8 | more;
        value >>= 7;
    }
    out
}

fn nested_inert_modules(count: usize) -> Vec<u8> {
    let total = UNIT * count;
    let mut out = Vec::with_capacity(total);
    for index in 0..count {
        out.extend_from_slice(b"\0asm\x01\0\0\0\0");
        out.extend_from_slice(&leb5(total - index * UNIT - 14));
        out.push(0);
    }
    out
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (at, byte)| {
            word | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            match at <= chunk.len() {
                true => out.push(char::from(ALPHABET[(word >> (18 - 6 * at) & 63) as usize])),
                false => out.push('='),
            }
        }
    }
    out
}

fn thread_cpu() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let nanos = stat.split_whitespace().next()?.parse().ok()?;
    Some(Duration::from_nanos(nanos))
}

fn timed(text: &str) -> (bool, Duration) {
    let (cpu, wall) = (thread_cpu(), std::time::Instant::now());
    let found = may_embed_wasm(text);
    let spent = match (cpu, thread_cpu()) {
        (Some(before), Some(after)) => after.saturating_sub(before),
        _ => wall.elapsed(),
    };
    (found, spent)
}

#[test]
fn nested_inert_base64_modules_are_scanned_in_linear_time() {
    let encoded = base64(&nested_inert_modules(NESTED));
    assert_eq!(encoded.matches("AGFzbQ").count(), NESTED);
    let text = format!("var s = \"{encoded}\";\nconsole.log(s.length);\n");

    let (found, spent) = timed(&text);

    assert!(!found, "an inert module does not trip the trigger");
    assert!(spent < BOUND, "{NESTED} nested modules took {spent:?}");
}

#[test]
fn nested_inert_byte_list_modules_are_scanned_in_linear_time() {
    let listed: Vec<String> = nested_inert_modules(NESTED / 4)
        .iter()
        .map(u8::to_string)
        .collect();
    let text = format!("var s = new Uint8Array([{}]);\n", listed.join(", "));
    assert_eq!(text.matches("0, 97, 115, 109").count(), NESTED / 4);

    let (found, spent) = timed(&text);

    assert!(!found, "an inert module does not trip the trigger");
    assert!(
        spent < BOUND,
        "{} nested modules took {spent:?}",
        NESTED / 4
    );
}

#[test]
fn a_spent_budget_stops_the_scan_and_counts_as_a_possible_module() {
    let encoded = base64(&nested_inert_modules(NESTED));
    let text = format!("var s = \"{encoded}\";\n");
    let scope = budget::enter(Limits {
        memory: None,
        deadline: Some(Deadline::after(Duration::ZERO)),
    });

    let (found, spent) = timed(&text);

    drop(scope);
    assert!(found, "a scan cut short cannot rule a module out");
    assert!(spent < BOUND, "{spent:?}");
    assert!(!may_embed_wasm(&text));
}
