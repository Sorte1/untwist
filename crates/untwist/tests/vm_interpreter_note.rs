use std::path::{Path, PathBuf};

use untwist::{Options, Output, Pipeline};

const NOTE: &str = "a bytecode interpreter (a virtual machine obfuscation) was found";

const FOUND: &[(&str, usize)] = &[
    ("tests/data/vm_interpreter/switch-vm.js", 1),
    ("tests/data/vm_interpreter/table-vm.js", 1),
    ("../../fixtures/obfuscators/vm/js-virtualizer-plain.js", 2),
];

const NOT_FOUND: &[&str] = &[
    "tests/data/vm_interpreter/tokenizer.js",
    "tests/data/vm_interpreter/no-bytecode.js",
    "tests/data/vm_interpreter/not-fetched.js",
    "tests/data/vm_interpreter/bounded-loop.js",
];

fn path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn run(rel: &str) -> Output {
    let source = std::fs::read_to_string(path(rel)).unwrap_or_else(|err| panic!("{rel}: {err}"));
    Pipeline::new(Options::default()).run(&source)
}

fn found(output: &Output) -> usize {
    output
        .notes
        .iter()
        .filter(|note| note.message.starts_with(NOTE))
        .count()
}

#[test]
fn a_bytecode_interpreter_loop_beside_its_bytecode_is_reported() {
    for (rel, count) in FOUND {
        assert_eq!(found(&run(rel)), *count, "{rel}");
    }
}

#[test]
fn a_tokenizer_or_a_loop_without_bytecode_is_never_called_an_interpreter() {
    for rel in NOT_FOUND {
        assert_eq!(found(&run(rel)), 0, "{rel}");
    }
}
