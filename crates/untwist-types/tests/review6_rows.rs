use untwist_types::{annotate, Mode, Refusal};

fn probe(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/probes/a40r5")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

const OPERATOR_TYPES: [&str; 8] = [
    ": number",
    ": bigint",
    ": string",
    "{number}",
    "{bigint}",
    "{string}",
    "number | bigint",
    "bigint | number",
];

#[test]
fn an_operator_on_an_object_operand_gets_no_type_in_either_mode() {
    let mut failures = Vec::new();
    for name in ["u1.mjs", "s11.mjs"] {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            let typed = out
                .lines()
                .filter(|line| !line.starts_with("console.log"))
                .any(|line| OPERATOR_TYPES.iter().any(|ty| line.contains(ty)));
            if typed {
                failures.push(format!("{name} {mode:?}\n{out}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn an_operator_on_primitive_operands_keeps_its_type() {
    let source = "const a = 2;\nconst b = a - 1;\nconst c = -a;\nlet d = a;\nd++;\nconst e = \"x\" + a;\nconsole.log(b, c, d, e);\nexport {};\n";
    let out = annotate(source, Mode::Ts).code;
    for typed in [
        "const b: number",
        "const c: number",
        "let d: number",
        "const e: string",
    ] {
        assert!(out.contains(typed), "{typed}\n{out}");
    }
}

#[test]
fn an_operator_on_an_unknown_operand_gets_no_type() {
    let source = "export function f(x) {\n  const y = x - 1;\n  const z = -x;\n  const w = x + \"\";\n  return [y, z, w];\n}\n";
    for mode in [Mode::Jsdoc, Mode::Ts] {
        let out = annotate(source, mode).code;
        assert!(
            !OPERATOR_TYPES.iter().any(|ty| out.contains(ty)),
            "{mode:?}\n{out}"
        );
    }
}

#[test]
fn a_file_with_an_object_operand_is_left_untyped_and_says_why() {
    for name in ["u1.mjs", "s11.mjs"] {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode);
            assert_eq!(out.refused, Some(Refusal::SymbolKey), "{name} {mode:?}");
            assert_eq!(out.code, source, "{name} {mode:?}");
        }
    }
}
