use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

const FROZEN: &str = include_str!("../tests/data/compile_records.sha256");
pub const FREEZE_DIR: &str = "UNTWIST_FREEZE_RECORDS";
const KEY_HEX: usize = 16;
const INPUT_KEPT: usize = 4096;

fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

const PROFILE: &str = match cfg!(debug_assertions) {
    true => "debug",
    false => "release",
};

pub fn frozen_key(scope: &str, input: &str) -> String {
    let mut key = digest(&format!("{scope}\n{PROFILE}\n{input}"));
    key.truncate(KEY_HEX);
    key
}

fn table() -> &'static HashMap<&'static str, Vec<&'static str>> {
    static TABLE: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: HashMap<&str, Vec<(usize, &str)>> = HashMap::new();
        for line in FROZEN.lines() {
            let mut fields = line.split(' ');
            let (Some(key), Some(index), Some(hash)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let index = index.parse().unwrap_or(usize::MAX);
            table.entry(key).or_default().push((index, hash));
        }
        table
            .into_iter()
            .map(|(key, mut units)| {
                units.sort_unstable();
                (key, units.into_iter().map(|(_, hash)| hash).collect())
            })
            .collect()
    })
}

pub fn frozen<T: Debug>(scope: &str, input: &str, units: &[T]) {
    let key = frozen_key(scope, input);
    let rendered: Vec<String> = units.iter().map(|unit| format!("{unit:?}")).collect();
    let hashes: Vec<String> = rendered.iter().map(|unit| digest(unit)).collect();
    match std::env::var_os(FREEZE_DIR) {
        Some(dir) => write_frozen(Path::new(&dir), &key, (scope, input), (&rendered, &hashes)),
        None => check_frozen(&key, scope, &rendered, &hashes),
    }
}

fn check_frozen(key: &str, scope: &str, rendered: &[String], hashes: &[String]) {
    let Some(expected) = table().get(key) else {
        panic!("{scope}: no frozen record under {key}; regenerate with cargo xtask freeze-records");
    };
    if expected.len() != hashes.len() {
        panic!(
            "{scope}: {} units against {} frozen under {key}: {rendered:#?}",
            hashes.len(),
            expected.len()
        );
    }
    let differing = expected
        .iter()
        .zip(hashes)
        .position(|(one, other)| one != other);
    if let Some(index) = differing {
        panic!(
            "{scope}: unit {index} differs from its frozen record under {key}: {:?}",
            rendered[index]
        );
    }
}

fn write_frozen(
    dir: &Path,
    key: &str,
    (scope, input): (&str, &str),
    (rendered, hashes): (&[String], &[String]),
) {
    let lines: String = hashes
        .iter()
        .enumerate()
        .map(|(index, hash)| format!("{key} {index} {hash}\n"))
        .collect();
    let at = dir.join("lines").join(key);
    if let Ok(earlier) = std::fs::read_to_string(&at) {
        assert_eq!(earlier, lines, "{scope}: two runs froze {key} differently");
        return;
    }
    let full = dir.join("full").join(format!("{key}.txt"));
    let kept: String = input.chars().take(INPUT_KEPT).collect();
    let text = format!(
        "{scope} ({PROFILE})\ninput {} bytes, sha256 {}\n{kept}\n\n{}\n",
        input.len(),
        digest(input),
        rendered.join("\n")
    );
    for (path, contents) in [(&full, text), (&at, lines)] {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the frozen records directory");
        }
        std::fs::write(path, contents).expect("a frozen record written");
    }
}
