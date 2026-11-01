mod common;

use oxc_allocator::Allocator;
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::rename::Renames;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, PassGroup};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::notes;
use untwist_deobfuscate::unstamp;

struct Retarget<'a> {
    from: &'static str,
    to: Ident<'a>,
}

impl<'a> VisitMut<'a> for Retarget<'a> {
    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if it.name == self.from {
            it.name = self.to;
        }
        walk_mut::walk_identifier_reference(self, it);
    }
}

fn nth_symbol(scoping: &Scoping, name: &str, nth: usize) -> SymbolId {
    scoping
        .symbol_ids()
        .filter(|symbol| scoping.symbol_name(*symbol) == name)
        .nth(nth)
        .unwrap()
}

struct Setup {
    source: &'static str,
    stamps: &'static [(&'static str, usize, &'static str)],
    retarget: Option<(&'static str, &'static str)>,
}

fn unstamped(setup: &Setup) -> (String, Vec<String>) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, setup.source);
    let sem = build_scoping(&parsed.program);
    let mut renames = Renames::new();
    for (name, nth, stamp) in setup.stamps {
        let symbol = nth_symbol(&sem.scoping, name, *nth);
        renames.insert(symbol, Ident::from(*stamp));
    }
    renames.apply(&mut parsed.program, &sem.scoping);
    if let Some((from, to)) = setup.retarget {
        let to = Ident::from(to);
        Retarget { from, to }.visit_program(&mut parsed.program);
    }
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::new(vec![unstamp::boxed()], common::pass_options());
    let (_sem, report) = run_once(&alloc, &mut parsed.program, sem, &mut group);
    let notes = report.notes.into_iter().map(|note| note.message).collect();
    (print(&parsed.program, PrintStyle::Pretty), notes)
}

fn blocked(notes: &[String]) -> bool {
    notes
        .iter()
        .any(|note| note == notes::UNSTAMP_BLOCKED || note == notes::UNSTAMP_FREE_REFERENCE)
}

#[test]
fn a_stamp_whose_name_is_reused_in_an_unrelated_scope_gets_its_name_back() {
    let (code, notes) = unstamped(&Setup {
        source:
            "function f() { var a = 1; return a; }\nfunction a() { return f(); }\nconsole.log(a());",
        stamps: &[("a", 1, "__STRING_ARRAY__")],
        retarget: None,
    });
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(code.contains("function a()"), "{code}");
    assert!(!blocked(&notes), "{notes:?}");
}

#[test]
fn a_stamp_whose_name_is_only_a_global_read_elsewhere_gets_its_name_back() {
    let (code, notes) = unstamped(&Setup {
        source: "(function () { function b() { return 1; } console.log(b()); })();\ntypeof b;",
        stamps: &[("b", 0, "__DECODE_0__")],
        retarget: None,
    });
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(!blocked(&notes), "{notes:?}");
}

#[test]
fn two_stamps_with_one_name_in_sibling_scopes_both_get_it_back() {
    let (code, notes) = unstamped(&Setup {
        source: "(function () { function b() { return 1; } b(); })();\n(function () { function b() { return 2; } b(); })();",
        stamps: &[("b", 0, "__DECODE_0__"), ("b", 1, "__DECODE_1__")],
        retarget: None,
    });
    assert!(!code.contains("__DECODE_"), "{code}");
    assert!(!blocked(&notes), "{notes:?}");
}

#[test]
fn two_stamps_with_one_name_in_nested_scopes_are_blocked() {
    let (code, notes) = unstamped(&Setup {
        source: "function b() { function b() { return 1; } return b(); }\nb();",
        stamps: &[("b", 0, "__DECODE_0__"), ("b", 1, "__DECODE_1__")],
        retarget: None,
    });
    assert!(code.contains("__DECODE_0__"), "{code}");
    assert!(code.contains("__DECODE_1__"), "{code}");
    assert!(blocked(&notes), "{notes:?}");
}

#[test]
fn a_stamp_whose_reference_a_local_of_its_name_would_capture_is_blocked() {
    let (code, notes) = unstamped(&Setup {
        source: "function b() { return 1; }\nfunction f() { var b = 2; return c() + b; }\nb();",
        stamps: &[("b", 0, "__DECODE_0__")],
        retarget: Some(("c", "__DECODE_0__")),
    });
    assert!(code.contains("function __DECODE_0__()"), "{code}");
    assert!(code.contains("return __DECODE_0__() + b"), "{code}");
    assert!(blocked(&notes), "{notes:?}");
}

#[test]
fn a_stamp_that_would_capture_an_outer_bindings_reference_is_blocked() {
    let (code, notes) = unstamped(&Setup {
        source: "var b = 1;\nfunction f() { function b() { return 2; } return b() + c; }\nf();",
        stamps: &[("b", 1, "__DECODE_0__")],
        retarget: Some(("c", "b")),
    });
    assert!(code.contains("function __DECODE_0__()"), "{code}");
    assert!(blocked(&notes), "{notes:?}");
}

#[test]
fn a_stamp_that_would_capture_a_global_read_inside_its_scope_is_blocked() {
    let (code, notes) = unstamped(&Setup {
        source: "function f() { function b() { return 2; } return b() + c; }\nf();",
        stamps: &[("b", 0, "__DECODE_0__")],
        retarget: Some(("c", "b")),
    });
    assert!(code.contains("function __DECODE_0__()"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note == notes::UNSTAMP_FREE_REFERENCE),
        "{notes:?}"
    );
}

#[test]
fn a_stamp_whose_scope_already_binds_its_name_is_blocked() {
    let (code, notes) = unstamped(&Setup {
        source: "function b() { return 1; }\nvar c = 2;\nb(c);",
        stamps: &[("b", 0, "__DECODE_0__"), ("c", 0, "b")],
        retarget: None,
    });
    assert!(code.contains("function __DECODE_0__()"), "{code}");
    assert!(code.contains("var b = 2"), "{code}");
    assert!(blocked(&notes), "{notes:?}");
}

#[test]
fn a_name_written_in_the_input_is_never_touched() {
    let (code, notes) = unstamped(&Setup {
        source: "var __DECODE_5__ = 7;\nfunction __STRING_ARRAY__() { return 1; }\nconsole.log(__DECODE_5__, __STRING_ARRAY__());",
        stamps: &[],
        retarget: None,
    });
    assert!(code.contains("var __DECODE_5__ = 7"), "{code}");
    assert!(code.contains("function __STRING_ARRAY__()"), "{code}");
    assert!(!blocked(&notes), "{notes:?}");
}

#[test]
fn a_stamp_declared_inside_a_with_body_gets_its_name_back() {
    let (code, notes) = unstamped(&Setup {
        source: "with ({}) { (function () { function b() { return 1; } b(); })(); }",
        stamps: &[("b", 0, "__DECODE_0__")],
        retarget: None,
    });
    assert!(code.contains("function b()"), "{code}");
    assert!(!blocked(&notes), "{notes:?}");
}

#[test]
fn a_stamp_whose_scope_nests_a_direct_eval_gets_its_name_back() {
    let (code, notes) = unstamped(&Setup {
        source: "function b() { return 1; }\nfunction f() { return (function () { return eval('1'); })(); }\nb(f());",
        stamps: &[("b", 0, "__DECODE_0__")],
        retarget: None,
    });
    assert!(code.contains("function b()"), "{code}");
    assert!(!blocked(&notes), "{notes:?}");
}
