mod common;

use common::stdin_output;

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Processed,
    ParseError,
    TypescriptRefusal,
}

fn outcome_of(source: &str) -> (Option<i32>, Outcome) {
    let out = stdin_output(source, &[]);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let outcome = match out.status.code() {
        Some(0) => Outcome::Processed,
        Some(3) if stderr.contains("uses TypeScript syntax") => Outcome::TypescriptRefusal,
        Some(1) => Outcome::ParseError,
        _ => panic!("{source:?}: {:?} {stderr}", out.status),
    };
    (out.status.code(), outcome)
}

#[test]
fn an_index_signature_after_a_written_or_escaped_modifier_is_refused_as_typescript() {
    use Outcome::*;
    let rows = [
        ("class C { static [k: T]; }\n", 3, TypescriptRefusal),
        ("class C { st\\u0061tic [k: T]; }\n", 3, TypescriptRefusal),
        ("class C { \\u0073tatic [k: T]; }\n", 3, TypescriptRefusal),
        ("class C { static [k: T]: U }\n", 3, TypescriptRefusal),
        ("class C { st\\u0061tic [k: T]: U }\n", 3, TypescriptRefusal),
        ("class C { readonly [k: T]; }\n", 3, TypescriptRefusal),
        ("class C { re\\u0061donly [k: T]; }\n", 3, TypescriptRefusal),
        ("class C { st\\u0061tic [k, j]; }\n", 3, TypescriptRefusal),
        (
            "class C { \\u0073tatic [k, j]() {} }\n",
            3,
            TypescriptRefusal,
        ),
        ("class C { static\n[k: T]; }\n", 3, TypescriptRefusal),
        ("class C { st\\u0061tic\n[k: T]; }\n", 3, TypescriptRefusal),
        ("class C { static async [k: T]; }\n", 3, TypescriptRefusal),
        ("class C { static [k]; }\n", 0, Processed),
        ("class C { st\\u0061tic [k]; }\n", 1, ParseError),
        ("class C { st\\u0061tic\n[k]; }\n", 1, ParseError),
        ("class C { st\\u0061tic [k] = 1 }\n", 1, ParseError),
    ];
    for (source, code, expected) in rows {
        assert_eq!(outcome_of(source), (Some(code), expected), "{source:?}");
    }
}
