use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const WORKERS: usize = 4;
const LANES: [(&str, bool); 2] = [("default", false), ("unsafe", true)];
const HEX_PREFIX: &str = "_0x";

const CONSOLE_DISABLE_KEPT: &str = "the default lane leaves the layers undecoded because of the console-disable callback: it writes toString and __proto__ on Function.prototype.bind results, which the alias check cannot tell from built-ins the rc4 decoder uses, and below an outer layer it reads the global object it holds by keys that layer still encodes";

const LAYERED_SAFE_UNIT_KEPT: &str = "the default lane leaves the outer unit of two layers of dead code, control flow flattening and rc4 undecoded, as main does, so neither layer decodes; the unsafe lane decodes both";

const SELF_DEFENDING_UNPROVEN: &str = "the default lane keeps a layer's self-defending block because the built-in scan cannot prove what its callback calls unpatched (another layer's protection or still-encoded machinery, or a method returning this, is in the program when the block is judged), so the layers it can see stay too";

const REFUSED_LAYERS: [(&str, &str, &str); 48] = [
    (
        "dead-cff-rc4-n2--classes-1.js",
        "default",
        LAYERED_SAFE_UNIT_KEPT,
    ),
    (
        "dead-cff-rc4-n2--functions-1.js",
        "default",
        LAYERED_SAFE_UNIT_KEPT,
    ),
    (
        "dead-cff-rc4-n2--greeting-1.js",
        "default",
        LAYERED_SAFE_UNIT_KEPT,
    ),
    (
        "dead-cff-rc4-n2--objects-1.js",
        "default",
        LAYERED_SAFE_UNIT_KEPT,
    ),
    (
        "dead-cff-rc4-n2--strings-1.js",
        "default",
        LAYERED_SAFE_UNIT_KEPT,
    ),
    (
        "dead-cff-rc4-n2--test-1.js",
        "default",
        LAYERED_SAFE_UNIT_KEPT,
    ),
    (
        "guarded-no-eval-n2--classes-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n3--classes-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n4--classes-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n2--functions-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n3--functions-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n4--functions-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n2--greeting-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n3--greeting-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n4--greeting-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n2--objects-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n3--objects-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n4--objects-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n2--strings-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n3--strings-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "guarded-no-eval-n4--strings-1.js",
        "default",
        CONSOLE_DISABLE_KEPT,
    ),
    (
        "preset-low-n2--classes-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n2--functions-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n2--greeting-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n2--objects-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n2--strings-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n3--classes-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n3--functions-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n3--greeting-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n3--objects-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n3--strings-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n4--classes-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n4--functions-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n4--greeting-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n4--objects-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    (
        "preset-low-n4--strings-1.js",
        "default",
        SELF_DEFENDING_UNPROVEN,
    ),
    ("self-n2--classes-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n2--greeting-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n2--objects-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n2--strings-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n3--classes-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n3--greeting-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n3--objects-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n3--strings-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n4--classes-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n4--greeting-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n4--objects-1.js", "default", SELF_DEFENDING_UNPROVEN),
    ("self-n4--strings-1.js", "default", SELF_DEFENDING_UNPROVEN),
];

#[derive(Clone)]
struct Row {
    file: String,
    set: String,
    layers: u32,
    program: String,
    node_judged: bool,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/obfuscator-relayer")
}

fn all_rows() -> Vec<Row> {
    let manifest = std::fs::read_to_string(fixtures().join("MANIFEST")).unwrap();
    manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let (set, layers) = fields[1].rsplit_once("-n").unwrap();
            Row {
                file: fields[0].to_string(),
                set: set.to_string(),
                layers: layers.parse().unwrap(),
                program: Path::new(fields[2])
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                node_judged: fields[5] == "node",
            }
        })
        .collect()
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn deobfuscated(row: &Row, unsafe_transforms: bool, rename: RenameMode) -> String {
    let source = std::fs::read_to_string(fixtures().join(&row.file)).unwrap();
    Pipeline::new(Options {
        unsafe_transforms,
        rename,
        ..Options::default()
    })
    .run(&source)
    .code()
    .to_string()
}

fn hex_name_end(bytes: &[u8], start: usize) -> Option<usize> {
    let boundary = |at: usize| {
        bytes
            .get(at)
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'_' || *b == b'$'))
    };
    let before_ok = start == 0 || boundary(start - 1);
    if !before_ok || !bytes[start..].starts_with(HEX_PREFIX.as_bytes()) {
        return None;
    }
    let digits = bytes[start + HEX_PREFIX.len()..]
        .iter()
        .take_while(|b| b.is_ascii_hexdigit())
        .count();
    let end = start + HEX_PREFIX.len() + digits;
    (digits > 0 && boundary(end)).then_some(end)
}

fn names_numbered(code: &str) -> String {
    let bytes = code.as_bytes();
    let mut numbers: BTreeMap<&str, usize> = BTreeMap::new();
    let mut out = String::with_capacity(code.len());
    let mut at = 0;
    let mut copied = 0;
    while at < bytes.len() {
        let Some(end) = hex_name_end(bytes, at) else {
            at += 1;
            continue;
        };
        out.push_str(&code[copied..at]);
        let next = numbers.len();
        let number = *numbers.entry(&code[at..end]).or_insert(next);
        out.push_str(&format!("{HEX_PREFIX}name{number}"));
        at = end;
        copied = end;
    }
    out.push_str(&code[copied..]);
    out
}

fn in_parallel<T: Sync>(items: &[T], run: impl Fn(&T) -> Vec<String> + Sync) -> Vec<String> {
    let chunk = items.len().div_ceil(WORKERS).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|part| {
                let run = &run;
                scope.spawn(move || part.iter().flat_map(run).collect::<Vec<_>>())
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    })
}

fn printed(node: &Path, path: &Path) -> String {
    match run(node, path).unwrap() {
        Some(out) => format!("{}exit {:?}", out.stdout, out.exit),
        None => "did not finish".to_string(),
    }
}

fn node_parity(rows: &[Row]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let judged: Vec<Row> = rows.iter().filter(|row| row.node_judged).cloned().collect();
    assert!(!judged.is_empty());
    let wrong = in_parallel(&judged, |row| {
        let expected = std::fs::read_to_string(
            fixtures()
                .join("expected")
                .join(format!("{}.stdout", row.program)),
        )
        .unwrap();
        let want = format!("{expected}exit Some(0)");
        let input = std::sync::OnceLock::new();
        LANES
            .iter()
            .filter_map(|(lane, unsafe_transforms)| {
                let path = dir.path().join(format!("{}.{lane}.js", row.file));
                let code = deobfuscated(row, *unsafe_transforms, RenameMode::None);
                std::fs::write(&path, code).unwrap();
                let got = printed(&node, &path);
                let same = got == want
                    || got == *input.get_or_init(|| printed(&node, &fixtures().join(&row.file)));
                (!same).then(|| {
                    format!(
                        "{} ({lane}): node printed {got:?}, the program prints {want:?}",
                        row.file
                    )
                })
            })
            .collect()
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn refused(row: &Row, lane: &str) -> bool {
    REFUSED_LAYERS
        .iter()
        .any(|(file, refused_lane, _)| *file == row.file && *refused_lane == lane)
}

fn single_pass_equality(rows: &[Row]) {
    let mut chains: BTreeMap<(String, String), Vec<Row>> = BTreeMap::new();
    for row in rows {
        chains
            .entry((row.set.clone(), row.program.clone()))
            .or_default()
            .push(row.clone());
    }
    let chains: Vec<Vec<Row>> = chains
        .into_values()
        .filter(|chain| chain.len() > 1)
        .collect();
    assert!(!chains.is_empty());
    let wrong = in_parallel(&chains, |chain| {
        let mut wrong = Vec::new();
        for (lane, unsafe_transforms) in LANES {
            let single = chain.iter().find(|row| row.layers == 1).unwrap();
            let want = names_numbered(&deobfuscated(single, unsafe_transforms, RenameMode::Mangle));
            for row in chain.iter().filter(|row| row.layers > 1) {
                let got = names_numbered(&deobfuscated(row, unsafe_transforms, RenameMode::Mangle));
                match (got == want, refused(row, lane)) {
                    (false, false) => wrong.push(format!(
                        "{} ({lane}): {} layers do not deobfuscate to what one layer does\n--- one layer\n{want}\n--- {} layers\n{got}",
                        row.file, row.layers, row.layers
                    )),
                    (true, true) => wrong.push(format!(
                        "{} ({lane}): now matches one layer; remove it from REFUSED_LAYERS",
                        row.file
                    )),
                    _ => {}
                }
            }
        }
        wrong
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn hex_names_are_numbered_by_first_appearance() {
    assert_eq!(
        names_numbered("var _0x1f = _0xab(_0x1f) + a_0x1 + _0x;"),
        "var _0xname0 = _0xname1(_0xname0) + a_0x1 + _0x;"
    );
}

#[test]
fn the_manifest_holds_every_layer_count_for_most_chains() {
    let rows = all_rows();
    assert!(rows.len() >= 200, "{}", rows.len());
    for layers in 1..=4 {
        assert!(
            rows.iter().any(|row| row.layers == layers),
            "no fixture with {layers} layers"
        );
    }
}

#[test]
fn every_relayered_fixture_runs_like_its_program_or_its_input_in_both_lanes() {
    node_parity(&all_rows());
}

#[test]
fn every_relayered_fixture_deobfuscates_to_what_one_layer_does_in_both_lanes() {
    single_pass_equality(&all_rows());
}
