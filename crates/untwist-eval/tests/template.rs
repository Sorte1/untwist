use oxc_allocator::Allocator;
use untwist_core::ast_util::matcher::statement_iife;
use untwist_core::parse::parse_program;
use untwist_eval::template::template_exact;

const TEMPLATE: &str = "(function (c, d) {
  function w(x) { return b(x); }
  var e = c();
  while (!![]) {
    try {
      var g = parseInt(w(0)) / 1;
      if (g === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 7);";

fn exact(source: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let (function, _) = statement_iife(&parsed.program.body[0]).unwrap();
    template_exact(&function.body.as_ref().unwrap().statements, "c", "d")
}

#[test]
fn the_obfuscator_template_is_exact() {
    assert!(exact(TEMPLATE));
    assert!(exact(&TEMPLATE.replace("while (!![])", "for (;;)")));
}

#[test]
fn any_other_statement_in_the_loop_makes_it_inexact() {
    let catch = "} catch (h) { e['push'](e['shift']()); }";
    let guard = TEMPLATE.replace(catch, &format!("{catch}\n    if (d > 0) break;"));
    assert!(!exact(&guard));
    let accumulate = TEMPLATE.replace("var g = parseInt(w(0)) / 1;", "var s = w(0); var g = s;");
    assert!(!exact(&accumulate));
    let after = TEMPLATE.replace("  }\n})(a, 7);", "  }\n  e.reverse();\n})(a, 7);");
    assert!(!exact(&after));
    let other = TEMPLATE.replace(
        "else e['push'](e['shift']());",
        "else x['push'](x['shift']());",
    );
    assert!(!exact(&other));
    let bounded = TEMPLATE.replace("while (!![])", "while (d > 0)");
    assert!(!exact(&bounded));
}

#[test]
fn a_rebound_array_local_or_expected_value_makes_it_inexact() {
    let rebound = TEMPLATE.replace(
        "var g = parseInt(w(0)) / 1;",
        "var g = (e = [1], parseInt(w(0)));",
    );
    assert!(!exact(&rebound));
    let written = TEMPLATE.replace(
        "var g = parseInt(w(0)) / 1;",
        "var g = (d = 9, parseInt(w(0)));",
    );
    assert!(!exact(&written));
    let catch_shadows = TEMPLATE.replace("} catch (h) {", "} catch (e) {");
    assert!(!exact(&catch_shadows));
    let accumulator = TEMPLATE
        .replace("var g = parseInt(w(0)) / 1;", "var e = parseInt(w(0)) / 1;")
        .replace("if (g === d)", "if (e === d)");
    assert!(!exact(&accumulator));
    let redeclared = TEMPLATE.replace("var e = c();", "var e = c();\n  var e = [];");
    assert!(!exact(&redeclared));
    let nested_write = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x) { d = x; return b(x); }",
    );
    assert!(!exact(&nested_write));
}

#[test]
fn a_wrapper_parameter_that_shadows_a_template_name_keeps_it_exact() {
    let shadow = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(c, d) { return b(c - 0, d); }",
    );
    assert!(exact(&shadow));
}

#[test]
fn a_loose_comparison_makes_it_inexact() {
    assert!(!exact(&TEMPLATE.replace("if (g === d)", "if (g == d)")));
}

#[test]
fn a_prelude_that_indexes_any_value_makes_it_inexact() {
    let indexed = TEMPLATE.replace("var e = c();", "var e = c();\n  var k = e[1];");
    assert!(!exact(&indexed));
    let other = TEMPLATE.replace("var e = c();", "var e = c();\n  var k = [e][0];");
    assert!(!exact(&other));
    let nested = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x) { return b(x)[0]; }",
    );
    assert!(exact(&nested));
}

#[test]
fn a_term_with_an_effect_makes_it_inexact() {
    let rotates = TEMPLATE.replace(
        "var g = parseInt(w(0)) / 1;",
        "var g = parseInt(w(0)) + (e.push(e.shift()), 0);",
    );
    assert!(!exact(&rotates));
    let writes = TEMPLATE.replace(
        "var g = parseInt(w(0)) / 1;",
        "var n = 0;\n      var g = parseInt(w(0)) + (n = 1);",
    );
    assert!(!exact(&writes));
    let local_function = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x) { return b(x); }\n  var z = function () { return 1; };",
    );
    assert!(!exact(
        &local_function.replace("parseInt(w(0))", "parseInt(z())")
    ));
    let impure_wrapper = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x) { e.push(e.shift()); return b(x); }",
    );
    assert!(!exact(&impure_wrapper));
    let parameter_callee = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x, b) { return b(x); }",
    );
    assert!(!exact(&parameter_callee));
}

#[test]
fn an_unwritten_alias_of_a_decoder_keeps_it_exact() {
    let alias = TEMPLATE
        .replace("var e = c();", "var j = b, e = c();")
        .replace("parseInt(w(0))", "parseInt(j(0))");
    assert!(exact(&alias));
    let rebound = alias.replace("var j = b, e = c();", "var j = b, e = c();\n  var j = w;");
    assert!(!exact(&rebound));
    let written = alias.replace(
        "function w(x) { return b(x); }",
        "function w(x) { j = x; return b(x); }",
    );
    assert!(!exact(&written));
    let shadowed = TEMPLATE
        .replace("var e = c();", "var j = b, e = c();")
        .replace(
            "function w(x) { return b(x); }",
            "function w(x, j) { return j(x); }",
        );
    assert!(!exact(&shadowed));
}

#[test]
fn an_identifier_operand_is_admitted_only_when_a_prelude_binds_it_to_an_unwritten_primitive() {
    let term = |prelude: &str, checksum: &str| {
        TEMPLATE
            .replace("var e = c();", &format!("var e = c();\n  {prelude}"))
            .replace("parseInt(w(0)) / 1", checksum)
    };
    assert!(exact(&term("var n = 3;", "parseInt(w(0)) + n")));
    assert!(exact(&term("var s = 'x';", "parseInt(w(0)) + s.length")));
    assert!(!exact(&term("var n = 3; n = 4;", "parseInt(w(0)) + n")));
    assert!(!exact(&term("var n = 3; var n = 4;", "parseInt(w(0)) + n")));
    assert!(!exact(&term(
        "var o = { valueOf: function () { return 0; } };",
        "parseInt(w(0)) + o"
    )));
    assert!(!exact(&term("var o = { v: 0 };", "parseInt(w(0)) + o.v")));
    assert!(!exact(&term("", "parseInt(w(0)) + d")));
    assert!(!exact(&term("", "parseInt(e[0]) + parseInt(w(1))")));
    assert!(!exact(&term("", "parseInt(w(0)) + undefined")));
    assert!(!exact(&term("var j = b;", "parseInt(w(0)) + j")));
}

#[test]
fn a_wrapper_parameter_is_its_argument() {
    let wrapped = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x, y) { return b(x - 0x10, y); }",
    );
    assert!(exact(
        &wrapped.replace("parseInt(w(0))", "parseInt(w(0x11, 'k'))")
    ));
    let free = TEMPLATE.replace(
        "function w(x) { return b(x); }",
        "function w(x) { return b(x - z); }",
    );
    assert!(!exact(&free));
}

fn dead_tail(source: &str) -> bool {
    use oxc_ast::ast::Statement;
    use untwist_eval::decode::checksum_from_rotator;
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("rotator");
    };
    let checksum = checksum_from_rotator(&statement.expression).expect("template");
    let mut terms = Vec::new();
    collect_terms(checksum.expression, &mut terms);
    untwist_eval::template::dead_tail_provable(checksum.body, &terms[1..], &["a", "b"])
}

fn collect_terms<'a>(
    expr: &'a oxc_ast::ast::Expression<'a>,
    out: &mut Vec<&'a oxc_ast::ast::Expression<'a>>,
) {
    use oxc_ast::ast::Expression;
    use oxc_syntax::operator::BinaryOperator;
    match expr {
        Expression::BinaryExpression(node) if node.operator == BinaryOperator::Addition => {
            collect_terms(&node.left, out);
            out.push(&node.right);
        }
        Expression::ParenthesizedExpression(node) => collect_terms(&node.expression, out),
        other => out.push(other),
    }
}

#[test]
fn a_skipped_term_is_provable_only_with_literal_decoder_arguments_and_call_free_tests() {
    let with = |checksum: &str| {
        let wrapped = TEMPLATE.replace(
            "function w(x) { return b(x); }",
            "function w(x, y) { return b(x - 0x10, y); }",
        );
        dead_tail(&wrapped.replace("parseInt(w(0)) / 1", checksum))
    };
    assert!(with(
        "parseInt(b(0)) + parseInt(b(1)) / 2 * -parseInt(w(-0x11, 'k'))"
    ));
    assert!(with("parseInt(b(0)) + parseInt(b(1 + 2))"));
    assert!(with("parseInt(b(0)) + (1 ? parseInt(b(2)) : 0)"));
    assert!(!with("parseInt(b(0)) + parseInt(b(parseInt(b(1))))"));
    assert!(!with("parseInt(b(0)) + parseInt(w(parseInt(b(1)), 'k'))"));
    assert!(!with(
        "parseInt(b(0)) + (b(1) === '4' ? parseInt(b(4)) : 0)"
    ));
    assert!(!with("parseInt(b(0)) + (parseInt(b(1)) || parseInt(b(4)))"));
    assert!(with("parseInt(b(parseInt(b(1)))) + parseInt(b(0))"));
}
