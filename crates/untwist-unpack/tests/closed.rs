mod common;

use common::parse;
use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrayExpressionElement, Expression, Statement};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::semantic::build_scoping;
use untwist_unpack::closed::{captured_by_factory, Allowed, Captured};
use untwist_unpack::matcher::{anonymous_factory, Factory};

const BUNDLE: &str = r#"
var shared = 1;
function helper() { return 2; }
var modules = [
  function (module, exports, require) { exports.a = shared; },
  function (module, exports, require) { exports.b = Math.max(1, 2) + typeof window + require("path").sep; },
  function (module, exports, require, extra) { exports.c = extra; },
  function (module, exports) { exports.d = arguments.length; },
  function (module) { module.exports = eval("shared"); },
  function (module, exports, require) { var shared = 2; exports.e = shared; function f() { return arguments; } exports.f = f; },
  function (module, exports, require) { exports.g = helper(); },
  (module, exports) => { exports.h = arguments; },
  function (module, exports, require = shared) { exports.i = require; },
  function (module, exports) { undeclared = 3; exports.j = undeclared; }
];
"#;

fn factories<'k, 'a>(program: &'k oxc_ast::ast::Program<'a>) -> Vec<Factory<'k, 'a>> {
    let Some(Statement::VariableDeclaration(declaration)) = program.body.last() else {
        panic!("the bundle ends in the module list");
    };
    let Some(Expression::ArrayExpression(array)) = &declaration.declarations[0].init else {
        panic!("the module list is an array");
    };
    array
        .elements
        .iter()
        .map(|element| match element {
            ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => {
                panic!("a factory")
            }
            element => anonymous_factory(element.to_expression()).expect("a factory"),
        })
        .collect()
}

fn verdicts(helpers: &[&str]) -> Vec<Option<Captured>> {
    let alloc = Allocator::default();
    let program = parse(&alloc, BUNDLE);
    let scoping = build_scoping(&program).scoping;
    let helpers: FxHashSet<SymbolId> = scoping
        .symbol_ids()
        .filter(|&id| helpers.contains(&scoping.symbol_name(id)))
        .collect();
    let allowed = Allowed {
        mapped_params: 3,
        helpers: &helpers,
    };
    factories(&program)
        .into_iter()
        .map(|factory| captured_by_factory(&scoping, factory, &allowed))
        .collect()
}

#[test]
fn a_factory_that_reads_a_bundle_binding_is_not_closed_and_host_globals_are() {
    assert_eq!(
        verdicts(&["helper"]),
        [
            Some(Captured::Binding("shared".to_string())),
            None,
            Some(Captured::Parameter("extra".to_string())),
            Some(Captured::Arguments),
            Some(Captured::Eval),
            None,
            None,
            Some(Captured::Arguments),
            Some(Captured::Binding("shared".to_string())),
            None,
        ]
    );
}

#[test]
fn a_helper_is_free_only_when_the_kind_moves_it() {
    assert_eq!(
        verdicts(&[])[6],
        Some(Captured::Binding("helper".to_string()))
    );
}

#[test]
fn the_refusal_names_what_the_factory_reads() {
    assert_eq!(
        Captured::Binding("shared".to_string()).describe(),
        "it reads shared, a binding of the bundle's scope"
    );
    assert_eq!(
        untwist_unpack::notes::factory_not_closed("3", Captured::Eval.describe()),
        "module 3 is not closed over its own scope; no bundle is written: it calls eval directly"
    );
}

#[test]
fn a_factory_holding_a_with_statement_is_not_closed() {
    let alloc = Allocator::default();
    let program = parse(
        &alloc,
        "var modules = [function (module, exports) { with (exports) { a = 1; } }];",
    );
    let scoping = build_scoping(&program).scoping;
    let helpers = FxHashSet::default();
    let allowed = Allowed {
        mapped_params: 3,
        helpers: &helpers,
    };

    let found: Vec<Option<String>> = factories(&program)
        .into_iter()
        .map(|factory| captured_by_factory(&scoping, factory, &allowed))
        .map(|captured| captured.map(|captured| captured.describe()))
        .collect();

    assert_eq!(found, [Some("it holds a with statement".to_string())]);
}
