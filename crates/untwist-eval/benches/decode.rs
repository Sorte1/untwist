use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, Criterion};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Program, Statement};
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{DecoderProgram, Evaluator, FnRef, InterpEvaluator};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("fixtures/reference/deobfuscate").join(name))
        .unwrap_or_default();
    std::fs::read_to_string(&path).unwrap_or_default()
}

fn function_named<'a>(program: &'a Program<'a>, name: &str) -> Option<FnRef<'a>> {
    for statement in &program.body {
        let Statement::FunctionDeclaration(function) = statement else {
            continue;
        };
        let Some(id) = &function.id else { continue };
        if id.name.as_str() == name {
            return Some(FnRef {
                name: id.name.as_str(),
                function: function.as_ref(),
                strict: false,
            });
        }
    }
    None
}

fn bench_case(c: &mut Criterion, label: &str, file: &str, array: &str, decoder: &str) {
    let source = fixture(file);
    if source.is_empty() {
        return;
    }
    c.bench_function(label, |b| {
        b.iter(|| {
            let alloc = Allocator::default();
            let parsed = parse_program(&alloc, &source);
            let (Some(string_array), Some(decoder_fn)) = (
                function_named(&parsed.program, array),
                function_named(&parsed.program, decoder),
            ) else {
                return;
            };
            let rotator = parsed
                .program
                .body
                .iter()
                .find_map(|statement| match statement {
                    Statement::ExpressionStatement(node) => Some(&node.expression),
                    _ => None,
                });
            let program = DecoderProgram {
                source_text: parsed.program.source_text,
                string_array,
                decoders: vec![decoder_fn],
                rotator,
                strict: false,
            };
            let mut evaluator = InterpEvaluator::new(Budget::default());
            let _ = evaluator.decode(&program, &[]);
        });
    });
}

fn bench_rotation(c: &mut Criterion, label: &str, file: &str, array: &str, decoder: &str) {
    let source = fixture(file);
    if source.is_empty() {
        return;
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let (Some(string_array), Some(decoder_fn)) = (
        function_named(&parsed.program, array),
        function_named(&parsed.program, decoder),
    ) else {
        return;
    };
    let rotator = parsed
        .program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::ExpressionStatement(node) => Some(&node.expression),
            _ => None,
        });
    let program = DecoderProgram {
        source_text: parsed.program.source_text,
        string_array,
        decoders: vec![decoder_fn],
        rotator,
        strict: false,
    };
    c.bench_function(label, |b| {
        b.iter(|| {
            let mut evaluator = InterpEvaluator::new(Budget::default());
            let Ok(mut session) = evaluator.instantiate(&program) else {
                return;
            };
            let _ = session.apply_rotation(&program);
        });
    });
}

const SESSION_ELEMENT_READS: usize = 10_000;

fn big_array_source(len: usize) -> String {
    let mut out = String::from("function a() { var h = [");
    for index in 0..len {
        out.push_str(&format!("'s{index}',"));
    }
    out.push_str("]; a = function () { return h; }; return a(); }");
    out
}

fn bench_session_element(c: &mut Criterion) {
    let source = big_array_source(20_000);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let Some(string_array) = function_named(&parsed.program, "a") else {
        return;
    };
    let program = DecoderProgram {
        source_text: parsed.program.source_text,
        string_array,
        decoders: Vec::new(),
        rotator: None,
        strict: false,
    };
    let mut evaluator = InterpEvaluator::new(Budget::default());
    let Ok(session) = evaluator.instantiate(&program) else {
        return;
    };
    c.bench_function("session_element/20k", |b| {
        b.iter(|| {
            for _ in 0..SESSION_ELEMENT_READS {
                let _ = session.array_first();
            }
        });
    });
}

fn benches(c: &mut Criterion) {
    bench_case(c, "decode/obfuscator.io", "obfuscator.io.js", "c", "d");
    bench_case(
        c,
        "decode/obfuscator.io-high",
        "obfuscator.io-high.js",
        "_0x4d6b",
        "_0x4c3e",
    );
    bench_rotation(c, "rotate/obfuscator.io", "obfuscator.io.js", "c", "d");
    bench_rotation(
        c,
        "rotate/obfuscator.io-high",
        "obfuscator.io-high.js",
        "_0x4d6b",
        "_0x4c3e",
    );
    bench_session_element(c);
}

criterion_group!(decode_benches, benches);
criterion_main!(decode_benches);
