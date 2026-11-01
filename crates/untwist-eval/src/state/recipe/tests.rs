use oxc_allocator::Allocator;
use oxc_ast::ast::{Function, Statement};
use oxc_span::Span;
use untwist_core::parse::parse_program;

use super::{compile_recipe, ContainerRef, GateRef, GateTest, KeyRecipe, KeyTerm};
use crate::budget::Budget;
use crate::interp::Interp;
use crate::state::collect::self_rewrite_target;
use crate::value::{JsString, PropKey, Value};

fn declared<'a>(statements: &'a [Statement<'a>], name: &str) -> &'a Function<'a> {
    statements
        .iter()
        .find_map(|statement| match statement {
            Statement::FunctionDeclaration(function)
                if function.id.as_ref().is_some_and(|id| id.name == name) =>
            {
                Some(function.as_ref())
            }
            _ => None,
        })
        .expect("the decoder is declared")
}

fn compiled(source: &str, outer: Option<&str>) -> (Option<KeyRecipe>, Vec<String>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = alloc.alloc(parsed.program);
    let mut interp = Interp::new_for_decode(program.source_text, Budget::default());
    let (function, name) = match outer {
        Some(name) => {
            let wrapper = declared(&program.body, name);
            let body = &wrapper.body.as_ref().expect("a body").statements;
            (
                self_rewrite_target(body, Some(name)).expect("an inner decoder"),
                name,
            )
        }
        None => (declared(&program.body, "D"), "D"),
    };
    let recipe = compile_recipe(function, name, &mut |text| interp.name_of(text));
    let texts = recipe
        .iter()
        .flat_map(|recipe| recipe.slice.iter())
        .map(|span| source[span.start as usize..span.end as usize].to_string())
        .collect();
    (recipe, texts)
}

fn literal(value: f64) -> Box<KeyTerm> {
    Box::new(KeyTerm::Literal(Value::Number(value)))
}

const HIGH: &str =
    include_str!("../../../../../fixtures/reference/deobfuscate/obfuscator.io-high.js");

#[test]
fn the_high_presets_inner_decoder_compiles_to_its_backward_slice() {
    let (recipe, texts) = compiled(HIGH, Some("_0x4c3e"));
    let recipe = recipe.expect("the high preset's decoder compiles");
    let gate = Span::new(185, 2653);
    let slice = [
        Span::new(107, 150),
        gate,
        Span::new(2657, 2707),
        Span::new(2708, 2737),
    ];
    assert_eq!(recipe.slice, slice);
    assert_eq!(texts[0], "_0x6cb730=_0x6cb730-(-0x1a7b+0x88b+0x130c);");
    assert!(texts[1].starts_with("if(_0x4c3e['GDNFei']===undefined){"));
    assert_eq!(
        texts[2],
        "_0x10ebcd=_0xaad546[-0x1e9f+0x1*-0x2dd+0x217c*0x1]"
    );
    assert_eq!(texts[3], "_0x50a375=_0x6cb730+_0x10ebcd");
    let index = KeyTerm::Sub(Box::new(KeyTerm::Argument(0)), literal(284.0));
    let key = KeyTerm::Add(
        Box::new(index.clone()),
        Box::new(KeyTerm::ArrayAt(literal(0.0))),
    );
    assert_eq!(recipe.key, key);
    assert_eq!(recipe.reads, [KeyTerm::ArrayAt(Box::new(index))]);
    let flag = PropKey::Name(JsString::from_str("GDNFei"));
    let test = GateTest::Undefined { loose: false };
    assert_eq!(
        recipe.gate,
        Some(GateRef {
            span: gate,
            flag,
            test
        })
    );
    assert!(matches!(recipe.container, ContainerRef::Binding(_)));
    assert!(recipe.array.is_some());
}

const BASE: &str = "\
function D(a, b) {
  a = a - 1;
  var r = E[a];
  if (D.flag === undefined) { D.flag = true; }
  EXTRA
  var s = E[0], k = a + s, v = M[k];
  if (!v) { r = 'x'; M[k] = r; } else r = v;
  return r;
}";

fn with_extra(extra: &str) -> Option<KeyRecipe> {
    compiled(&BASE.replace("EXTRA", extra), None).0
}

fn with_key(key: &str) -> Option<KeyRecipe> {
    compiled(&BASE.replace("EXTRA", "").replace("k = a + s", key), None).0
}

#[test]
fn the_synthetic_decoder_compiles_with_nothing_extra() {
    let recipe = with_extra("").expect("the base decoder compiles");
    let key = KeyTerm::Add(
        Box::new(KeyTerm::Sub(Box::new(KeyTerm::Argument(0)), literal(1.0))),
        Box::new(KeyTerm::ArrayAt(literal(0.0))),
    );
    assert_eq!(recipe.key, key);
    assert_eq!(recipe.reads.len(), 1);
    assert!(with_extra("var t = a + 1;").is_some());
    assert!(with_extra("var t = E[a + 1];").is_some());
    assert!(with_key("k = a - 2 + s").is_some());
}

#[test]
fn a_call_in_the_slice_refuses_the_recipe() {
    assert!(with_key("k = a + g(s)").is_none());
    assert!(with_key("k = a + E[h()]").is_none());
}

#[test]
fn a_member_read_off_anything_but_the_array_refuses_the_recipe() {
    assert!(with_key("k = a + o.p").is_none());
    assert!(with_key("k = a + o[0]").is_none());
    assert!(with_key("k = a + E.length").is_none());
}

#[test]
fn a_write_to_a_slice_variable_outside_the_slice_refuses_the_recipe() {
    assert!(with_extra("a = 3;").is_none());
    assert!(with_extra("var a = 3;").is_none());
    let late = BASE
        .replace("EXTRA", "")
        .replace("k = a + s, v = M[k]", "k = a + s; s = 2; var v = M[k]");
    assert!(compiled(&late, None).0.is_none());
}

#[test]
fn an_effect_outside_the_slice_refuses_the_recipe() {
    assert!(with_extra("foo();").is_none());
    assert!(with_extra("var t = o.p;").is_none());
    assert!(with_extra("var u = E[a].x;").is_none());
    assert!(with_extra("new F();").is_none());
    assert!(with_extra("g.h = 1;").is_none());
    assert!(with_extra("var w = this;").is_none());
}

#[test]
fn a_gate_that_is_not_on_the_decoder_or_has_an_else_refuses_the_recipe() {
    assert!(with_extra("if (G.flag === undefined) { G.flag = 1; }").is_none());
    assert!(with_extra("if (D.other === undefined) {} else {}").is_none());
}

const RC4: &str = include_str!("../../../../../fixtures/eval/rc4-rot5-shift1cc.js");

#[test]
fn a_memo_on_the_decoder_behind_a_ternary_return_compiles() {
    let (recipe, texts) = compiled(RC4, Some("b"));
    let recipe = recipe.expect("the rc4 decoder compiles");
    let cache = PropKey::Name(JsString::from_str("cache"));
    assert_eq!(recipe.container, ContainerRef::SelfMember(cache));
    assert_eq!(texts.len(), 4);
    assert_eq!(texts[0], "f = f - 460;");
    assert!(texts[1].starts_with("if (b['init'] === undefined) {"));
    assert_eq!(texts[2], "first = e[0]");
    assert_eq!(texts[3], "key = f + first");
}

#[test]
fn a_ternary_return_that_does_not_copy_the_hit_refuses_the_recipe() {
    let changed = RC4.replace(": (i = hit), i;", ": (i = hit + 1), i;");
    assert!(compiled(&changed, Some("b")).0.is_none());
    let reversed = RC4.replace("return !hit ?", "return hit ?");
    assert!(compiled(&reversed, Some("b")).0.is_none());
}

#[test]
fn a_conditional_statement_tail_after_sequence_splitting_compiles() {
    let split = RC4
        .replace(
            "var first = e[0], key = f + first, hit = b['cache'][key];",
            "var first = e[0];\nvar key = f + first;\nvar hit = b['cache'][key];",
        )
        .replace(
            "return !hit ? ((i = b['decoder'](i, g)), (b['cache'][key] = i)) : (i = hit), i;",
            "!hit ? ((i = b['decoder'](i, g)), (b['cache'][key] = i)) : (i = hit);\nreturn i;",
        );
    assert!(compiled(&split, Some("b")).0.is_some());
    let other = split.replace("return i;", "return first;");
    assert!(compiled(&other, Some("b")).0.is_none());
}

const FUNCTION_SHADOWS_PARAMETER: &str =
    include_str!("../../../../../fixtures/eval/rc4-rot5-shift1cc.js");

fn named_b(source: &str) -> Option<KeyRecipe> {
    compiled(source, Some("b")).0
}

#[test]
fn a_function_declaration_anywhere_in_the_decoder_refuses_the_recipe() {
    assert!(named_b(FUNCTION_SHADOWS_PARAMETER).is_some());
    let at_entry =
        FUNCTION_SHADOWS_PARAMETER.replace("f = f - 460;", "f = f - 460;\nfunction f() { }");
    assert!(named_b(&at_entry).is_none());
    let in_gate = FUNCTION_SHADOWS_PARAMETER.replace(
        "b['init'] = true;",
        "b['init'] = true;\nfunction first() { }",
    );
    assert!(named_b(&in_gate).is_none());
    let unrelated = FUNCTION_SHADOWS_PARAMETER.replace(
        "b['init'] = true;",
        "b['init'] = true;\nfunction other() { }",
    );
    assert!(named_b(&unrelated).is_none());
    let counter_example = "function a() { return []; }\nfunction b(c, d) {\n  var e = a();\n  return (b = function (f, g) {\n    g = g - 0;\n    function f() { }\n    if (b.k === undefined) { c = arguments; b.k = true; }\n    var n = f + g + e[0], hit = c[n];\n    var v;\n    if (!hit) { v = e[g]; c[n] = v; } else v = hit;\n    return v;\n  }), b(c, d);\n}";
    assert!(named_b(counter_example).is_none());
}

#[test]
fn a_slice_write_to_a_name_a_nested_scope_also_binds_refuses_the_recipe() {
    let counter_example = "function a() { return []; }\nfunction b(c, d) {\n  var e = a(), x = 'u';\n  return (b = function (f, g) {\n    f = f - 0;\n    if (b.k === undefined) { c = arguments; b.k = true; try { } catch (x) { } }\n    x = e[2];\n    var n = f + e[0], hit = c[n];\n    var v;\n    if (!hit) { v = e[f] + x; c[n] = v; } else v = hit;\n    return v;\n  }), b(c, d);\n}";
    assert!(named_b(counter_example).is_none());
    assert!(named_b(
        &counter_example
            .replace("x = e[2];", "var x = e[2];")
            .replace("catch (x)", "catch (y)")
    )
    .is_some());
    assert!(with_extra("{ let t = 1; } var t = a;").is_none());
    assert!(with_extra("var t = a; for (let t = 0; t < 1; t++) {}").is_none());
    assert!(with_extra("var t = a; try { } catch ({ t }) { }").is_none());
    assert!(with_extra("var t = a;").is_some());
}

#[test]
fn the_hit_copy_must_target_a_parameter_or_a_var() {
    let named = FUNCTION_SHADOWS_PARAMETER
        .replace(
            "return (b = function (f, g) {",
            "return (b = function q(f, g) {",
        )
        .replace("(i = hit), i;", "(q = hit), q;");
    assert!(named_b(&named).is_none());
    let let_bound = BASE
        .replace("EXTRA", "")
        .replace("var r = E[a];", "let r = E[a];");
    assert!(compiled(&let_bound, None).0.is_none());
}
