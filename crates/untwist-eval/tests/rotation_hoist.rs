mod common;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program, Statement};
use untwist_core::parse::parse_program;
use untwist_eval::decode::{checksum_from_rotator, InterpEvaluator};

fn first_expression<'a>(program: &'a Program<'a>) -> Option<&'a Expression<'a>> {
    program.body.iter().find_map(|statement| match statement {
        Statement::ExpressionStatement(node) => Some(&node.expression),
        _ => None,
    })
}

#[test]
fn a_prelude_that_never_indexes_the_array_is_searched() {
    let alloc = Allocator::default();
    let source = untwist_testing::read_fixture("deobfuscate", "obfuscator.io.js");
    let parsed = parse_program(&alloc, &source);
    let rotator = first_expression(&parsed.program).expect("rotator");
    assert!(checksum_from_rotator(rotator).is_some());
}

#[test]
fn a_prelude_that_indexes_the_array_is_not_searched() {
    let alloc = Allocator::default();
    let source = "(function (c, d) {\n  var f = c();\n  var first = f[0];\n  while (!![]) {\n    try {\n      var g = parseInt(first);\n      if (g === d) break;\n      else f.push(f.shift());\n    } catch (h) { f.push(f.shift()); }\n  }\n})(__STRING_ARRAY__, 1);";
    let parsed = parse_program(&alloc, source);
    let rotator = first_expression(&parsed.program).expect("rotator");
    assert!(checksum_from_rotator(rotator).is_none());
}

#[test]
fn a_generator_or_async_rotator_is_not_searched() {
    let source = "(function (c, d) {\n  var f = c();\n  while (!![]) {\n    try {\n      var g = parseInt(b(0));\n      if (g === d) break;\n      else f.push(f.shift());\n    } catch (h) { f.push(f.shift()); }\n  }\n})(__STRING_ARRAY__, 1);";
    for form in ["(function* (c, d)", "(async function (c, d)"] {
        let alloc = Allocator::default();
        let changed = source.replace("(function (c, d)", form);
        let parsed = parse_program(&alloc, &changed);
        let rotator = first_expression(&parsed.program).expect("rotator");
        assert!(checksum_from_rotator(rotator).is_none(), "{form}");
    }
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let rotator = first_expression(&parsed.program).expect("rotator");
    assert!(checksum_from_rotator(rotator).is_some());
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high-preset rotation search is release-only"
)]
fn hoisting_the_prelude_does_not_change_the_rotation_found() {
    for (file, array, decoder) in [
        ("obfuscator.io.js", "c", "d"),
        ("obfuscator.io-high.js", "_0x4d6b", "_0x4c3e"),
    ] {
        let alloc = Allocator::default();
        let source = untwist_testing::read_fixture("deobfuscate", file);
        let parsed = parse_program(&alloc, &source);
        let program = common::decoder_program(&parsed.program, array, &[decoder], Some(0));
        let mut evaluator = InterpEvaluator::default();
        let mut session = evaluator.instantiate(&program).expect("instantiates");
        let found = session.apply_rotation(&program).expect("no bail");
        let expected = if file == "obfuscator.io-high.js" {
            Some(338)
        } else {
            found
        };
        assert_eq!(found, expected, "{file}");
        assert!(found.is_some(), "{file}: the checksum search must succeed");
    }
}

#[test]
fn the_high_presets_prelude_is_the_one_the_hoisted_path_takes() {
    let alloc = Allocator::default();
    let source = untwist_testing::read_fixture("deobfuscate", "obfuscator.io-high.js");
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "_0x4d6b", &["_0x4c3e"], Some(0));
    let rotator = program.rotator.expect("rotator");
    let checksum = checksum_from_rotator(rotator).expect("checksum");
    assert_eq!(checksum.prelude.len(), 6);
}

#[test]
fn the_hoisted_scope_carries_the_prelude_bindings_into_every_attempt() {
    let alloc = Allocator::default();
    let source = "(function (c, d) {\n  var f = c();\n  var bump = 1;\n  while (!![]) {\n    try {\n      var g = parseInt(b(0)) + bump;\n      if (g === d) break;\n      else f.push(f.shift());\n    } catch (h) { f.push(f.shift()); }\n  }\n})(__STRING_ARRAY__, 1);";
    let parsed = parse_program(&alloc, source);
    let rotator = first_expression(&parsed.program).expect("rotator");
    let checksum = checksum_from_rotator(rotator).expect("checksum");
    assert_eq!(checksum.prelude.len(), 2);
}
