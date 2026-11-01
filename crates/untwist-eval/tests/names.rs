use std::collections::{HashMap, HashSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, IdentifierReference, Program, Statement};
use untwist_core::parse::parse_program;
use untwist_eval::env::{child_env, declare, root_env};
use untwist_eval::frame::Frame;
use untwist_eval::names::{Name, Names};
use untwist_eval::value::Value;

fn expression_identifiers<'a>(program: &'a Program<'a>) -> Vec<&'a IdentifierReference<'a>> {
    program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ExpressionStatement(node) => match &node.expression {
                Expression::Identifier(id) => Some(id.as_ref()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn equal_texts_intern_to_one_name_and_different_texts_to_two() {
    let mut names = Names::default();
    let first = names.intern("_0x3a8f2c");
    let again = names.intern(&String::from("_0x3a8f2c").leak()[..]);
    let other = names.intern("_0x3a8f2d");
    assert_eq!(first, again);
    assert_ne!(first, other);
}

#[test]
fn an_inner_binding_shadows_an_outer_one_by_name() {
    let mut names = Names::default();
    let e = names.intern("e");
    let outer = root_env();
    declare(&outer, e, Value::Number(1.0));
    let inner = child_env(&outer);
    declare(&inner, e, Value::Number(2.0));
    assert!(matches!(Frame::slot_by_name(&inner, e), Ok(Some(Value::Number(n))) if n == 2.0));
    assert!(matches!(Frame::slot_by_name(&outer, e), Ok(Some(Value::Number(n))) if n == 1.0));
}

fn leaked(text: &str) -> &'static str {
    String::from(text).leak()
}

#[test]
fn a_text_has_one_name_whether_it_is_met_first_inside_or_outside_the_source() {
    let source: &'static str = "var abc = abcd + abc; ab(abc)";
    let mut names = Names::for_source(source);
    let outside_first = names.intern(leaked("abc"));
    assert_eq!(names.intern(&source[4..7]), outside_first);
    assert_eq!(names.intern(&source[17..20]), outside_first);
    let inside_first = names.intern(&source[10..14]);
    assert_eq!(names.intern(leaked("abcd")), inside_first);
    let prefix = names.intern(&source[22..24]);
    assert_eq!(names.intern(leaked("ab")), prefix);
    assert_ne!(prefix, outside_first);
    assert_ne!(prefix, inside_first);
    assert_ne!(outside_first, inside_first);
}

#[test]
fn a_slice_that_stops_inside_an_identifier_is_still_its_own_text() {
    let source: &'static str = "abcdef; abc";
    let mut names = Names::for_source(source);
    let cut = names.intern(&source[0..3]);
    let whole = names.intern(&source[0..6]);
    let later = names.intern(&source[8..11]);
    assert_eq!(cut, later);
    assert_ne!(cut, whole);
    assert_eq!(names.intern(leaked("abcdef")), whole);
}

#[test]
fn texts_that_are_not_ascii_identifiers_intern_consistently() {
    let source: &'static str = "var ünï = 1, $_9 = 2;";
    let mut names = Names::for_source(source);
    let accented = names.intern(&source[4..9]);
    assert_eq!(names.intern(leaked("ünï")), accented);
    let symbols = names.intern(&source[15..18]);
    assert_eq!(names.intern(leaked("$_9")), symbols);
    assert_ne!(accented, symbols);
    assert_eq!(names.intern(""), names.intern(leaked("")));
}

#[test]
fn every_realm_global_and_arguments_have_a_fixed_name_that_takes_no_table_entry() {
    use untwist_eval::builtins::Realm;
    let (_realm, globals) = Realm::with_globals();
    let mut names = Names::for_source("Math(arguments)");
    let env = root_env();
    globals.install(&env, &mut names);
    assert!(names.is_empty());
    let arguments = names.intern("arguments");
    let math = names.intern(leaked("Math"));
    assert_ne!(arguments, math);
    assert!(names.is_empty());
    assert!(matches!(Frame::slot_by_name(&env, math), Ok(Some(_))));
}

#[test]
fn a_binding_keyed_by_a_name_is_a_word_smaller_than_one_keyed_by_its_text() {
    use std::mem::size_of;
    assert_eq!(size_of::<Name>(), 8);
    assert_eq!(size_of::<(Name, Value<'static>)>(), 24);
    assert_eq!(size_of::<(&str, Value<'static>)>(), 32);
}

const STRESS_PARSES: usize = 5;
const STRESS_STATEMENTS: usize = 3000;
const STRESS_NAMES: usize = 700;

fn stress_source(parse: usize) -> String {
    (0..STRESS_STATEMENTS)
        .map(|index| format!("v{parse}_{:03};", index % STRESS_NAMES))
        .collect()
}

fn stress_orders() -> Vec<Vec<(usize, usize)>> {
    let by_parse: Vec<(usize, usize)> = (0..STRESS_PARSES)
        .flat_map(|parse| (0..STRESS_STATEMENTS).map(move |index| (parse, index)))
        .collect();
    let by_statement: Vec<(usize, usize)> = (0..STRESS_STATEMENTS)
        .flat_map(|index| (0..STRESS_PARSES).map(move |parse| (parse, index)))
        .collect();
    let reversed: Vec<(usize, usize)> = by_statement.iter().rev().copied().collect();
    let strided: Vec<(usize, usize)> = (0..by_parse.len())
        .map(|step| by_parse[(step * 7919) % by_parse.len()])
        .collect();
    vec![by_parse, by_statement, reversed, strided]
}

#[test]
fn every_identifier_text_interns_to_one_name_in_any_order_across_parses() {
    let sources: Vec<String> = (0..STRESS_PARSES).map(stress_source).collect();
    let allocator = Allocator::default();
    let parsed: Vec<_> = sources
        .iter()
        .map(|source| parse_program(&allocator, source))
        .collect();
    let nodes: Vec<Vec<&IdentifierReference<'_>>> = parsed
        .iter()
        .map(|result| expression_identifiers(&result.program))
        .collect();
    for (parse, found) in nodes.iter().enumerate() {
        assert_eq!(found.len(), STRESS_STATEMENTS, "parse {parse}");
        assert_eq!(found[1].span, nodes[0][1].span);
    }
    let mut names = Names::for_source(parsed[0].program.source_text);
    let mut first: HashMap<&str, Name> = HashMap::new();
    let mut checked = 0usize;
    for order in stress_orders() {
        for (parse, index) in order {
            let text = nodes[parse][index].name.as_str();
            let name = names.intern(text);
            assert_eq!(*first.entry(text).or_insert(name), name, "{text}");
            checked += 1;
        }
    }
    assert_eq!(checked, STRESS_PARSES * STRESS_STATEMENTS * 4);
    assert_eq!(first.len(), STRESS_PARSES * STRESS_NAMES);
    let distinct: HashSet<Name> = first.values().copied().collect();
    assert_eq!(distinct.len(), first.len());
}
