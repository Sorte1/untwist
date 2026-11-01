mod common;

use oxc_ast::ast::Statement;
use untwist_eval::bail::{AbruptKind, Bail};
use untwist_eval::env::{
    assign, child_env, declare, has_binding, hoist_function_declarations, hoist_var_names,
    read_binding, root_env, Assigned, ScopeChainTooDeep, SCOPE_CHAIN_CAP,
};
use untwist_eval::frame::{Frame, FrameRef};
use untwist_eval::names::Names;
use untwist_eval::value::Value;

#[test]
fn assignment_writes_in_the_owning_scope_so_closures_see_it() {
    let mut names = Names::default();
    let d = names.intern("d");
    let missing = names.intern("missing");
    let outer = root_env();
    declare(&outer, d, Value::Number(1.0));
    let inner = child_env(&outer);
    assert_eq!(assign(&inner, d, Value::Number(2.0)), Ok(Assigned::Written));
    assert!(matches!(Frame::slot_by_name(&outer, d), Ok(Some(Value::Number(n))) if n == 2.0));
    assert_eq!(
        assign(&inner, missing, Value::Number(0.0)),
        Ok(Assigned::Unbound)
    );
    assert_eq!(has_binding(&inner, missing), Ok(false));
}

#[test]
fn shadowing_stops_the_walk() {
    let mut names = Names::default();
    let e = names.intern("e");
    let outer = root_env();
    declare(&outer, e, Value::Number(1.0));
    let inner = child_env(&outer);
    declare(&inner, e, Value::Number(2.0));
    assert!(matches!(Frame::slot_by_name(&inner, e), Ok(Some(Value::Number(n))) if n == 2.0));
    assert!(matches!(Frame::slot_by_name(&outer, e), Ok(Some(Value::Number(n))) if n == 1.0));
}

#[test]
fn destructuring_patterns_are_skipped_not_bound() {
    let source = "var {a, b} = obj; var [c, d] = arr; var plain = 1;";
    common::with_program(source, |program| {
        let mut names = Vec::new();
        assert!(hoist_var_names(&program.body, &mut names).is_ok());
        assert_eq!(names, vec!["plain"]);
    });
}

#[test]
fn var_names_hoist_out_of_blocks_but_not_out_of_functions() {
    let source = "var a; if (x) { var b; } for (var c = 0;;) {} try { var d; } catch (e) { var f; } function g() { var hidden; }";
    common::with_program(source, |program| {
        let mut names = Vec::new();
        assert!(hoist_var_names(&program.body, &mut names).is_ok());
        names.sort_unstable();
        assert_eq!(names, vec!["a", "b", "c", "d", "f"]);
    });
}

#[test]
fn function_declarations_are_collected_in_source_order() {
    let source = "function i(c, d) { return 1; } function h(c, d) { return 2; } var x = 1;";
    common::with_program(source, |program| {
        let decls = hoist_function_declarations(&program.body);
        let names: Vec<&str> = decls.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec!["i", "h"]);
    });
}

#[test]
fn has_binding_reports_bindings_visible_through_the_scope_chain() {
    let mut names = Names::default();
    let g = names.intern("g");
    let not_declared = names.intern("not_declared");
    let outer = root_env();
    declare(&outer, g, Value::Number(1.0));
    let inner = child_env(&outer);
    assert_eq!(has_binding(&inner, g), Ok(true));
    assert_eq!(has_binding(&outer, not_declared), Ok(false));
}

#[test]
fn root_env_has_no_parent_and_child_env_links_to_its_creator() {
    let outer = root_env();
    assert!(outer.borrow().parent.is_none());
    let inner = child_env(&outer);
    assert!(inner.borrow().parent.is_some());
    let mut names = Names::default();
    let h = names.intern("h");
    declare(&outer, h, Value::Number(9.0));
    assert!(matches!(Frame::slot_by_name(&inner, h), Ok(Some(Value::Number(n))) if n == 9.0));
}

#[test]
fn scope_chain_cap_bounds_the_lookup_walk() {
    assert_eq!(untwist_eval::env::SCOPE_CHAIN_CAP, 256);
    let mut names = Names::default();
    let far = names.intern("far");
    let root = root_env();
    declare(&root, far, Value::Number(1.0));
    let mut beyond_cap = root.clone();
    for _ in 0..untwist_eval::env::SCOPE_CHAIN_CAP + 10 {
        beyond_cap = child_env(&beyond_cap);
    }
    assert!(matches!(
        Frame::slot_by_name(&beyond_cap, far),
        Err(ScopeChainTooDeep)
    ));
    assert_eq!(
        assign(&beyond_cap, far, Value::Number(2.0)),
        Err(untwist_eval::bail::Bail::ChainTooDeep(&"scope"))
    );
    assert_eq!(
        has_binding(&beyond_cap, far),
        Err(untwist_eval::bail::Bail::ChainTooDeep(&"scope"))
    );

    let mut within_cap = root.clone();
    for _ in 0..untwist_eval::env::SCOPE_CHAIN_CAP - 10 {
        within_cap = child_env(&within_cap);
    }
    assert!(
        matches!(Frame::slot_by_name(&within_cap, far), Ok(Some(Value::Number(n))) if n == 1.0)
    );
}

#[test]
fn a_scope_chain_deeper_than_the_cap_bails_instead_of_shadowing_the_outer_binding() {
    let depth = untwist_eval::env::SCOPE_CHAIN_CAP + 10;
    let source = format!(
        "var a = 1; {} a = 2; {} a",
        "{ let b = 1;".repeat(depth),
        "}".repeat(depth)
    );
    assert_eq!(
        common::eval(&source),
        Err("scope chain too deep".to_string())
    );
}

#[test]
fn nested_blocks_that_declare_nothing_add_no_link_to_the_scope_chain() {
    let depth = untwist_eval::env::SCOPE_CHAIN_CAP + 10;
    let source = format!(
        "var a = 1; {} a = 2; {} a",
        "{".repeat(depth),
        "}".repeat(depth)
    );
    assert_eq!(common::eval(&source), Ok("number:2".to_string()));
}

#[test]
fn a_block_without_declarations_binds_nothing_new() {
    assert_eq!(
        common::eval("var a = 1; { var a = 2; a = a + 1; } a"),
        Ok("number:3".to_string())
    );
    assert_eq!(
        common::eval("var f; { f = function () { return typeof missing; }; } f()"),
        Ok("string:undefined".to_string())
    );
}

#[test]
fn a_block_with_a_let_still_shadows() {
    assert_eq!(
        common::eval("var a = 1; { let a = 2; } a"),
        Ok("number:1".to_string())
    );
}

#[test]
fn a_scope_that_grows_past_the_small_form_keeps_every_binding() {
    let source = (0..40)
        .map(|index| format!("var v{index} = {index};"))
        .collect::<String>()
        + "v0 + v39";
    assert_eq!(common::eval(&source), Ok("number:39".to_string()));
    let mut names = Names::default();
    let root = root_env();
    for index in 0..40 {
        let name: &'static str = Box::leak(format!("n{index}").into_boxed_str());
        declare(&root, names.intern(name), Value::Number(f64::from(index)));
    }
    let n0 = names.intern("n0");
    let n39 = names.intern("n39");
    let n7 = names.intern("n7");
    assert!(matches!(Frame::slot_by_name(&root, n0), Ok(Some(Value::Number(n))) if n == 0.0));
    assert!(matches!(Frame::slot_by_name(&root, n39), Ok(Some(Value::Number(n))) if n == 39.0));
    assert_eq!(
        assign(&root, n7, Value::Number(70.0)),
        Ok(Assigned::Written)
    );
    assert!(matches!(Frame::slot_by_name(&root, n7), Ok(Some(Value::Number(n))) if n == 70.0));
}

#[test]
fn an_implicit_global_assigned_in_a_block_outlives_the_block() {
    assert_eq!(
        common::eval("{ g = 1; } typeof g"),
        Ok("string:number".to_string())
    );
}

#[test]
fn every_declaring_form_keeps_the_blocks_scope_and_none_of_them_leaks() {
    assert_eq!(
        common::eval("{ let x = 1; } typeof x"),
        Ok("string:undefined".to_string())
    );
    assert_eq!(
        common::eval("{ const y = 1; } typeof y"),
        Ok("string:undefined".to_string())
    );
    assert_eq!(
        common::eval("'use strict'; { function f() { return 1; } } typeof f"),
        Ok("string:undefined".to_string())
    );
    common::with_program("{ class C {} }", |program| {
        assert!(matches!(
            program.body.first(),
            Some(Statement::BlockStatement(block))
                if matches!(block.body.first(), Some(Statement::ClassDeclaration(_)))
        ));
    });
    assert_eq!(
        common::eval("{ class C {} } typeof C"),
        Err("unsupported construct: statement kind".to_string())
    );
}

#[test]
fn an_implicit_global_escapes_a_block_whether_or_not_a_class_declaration_keeps_its_scope() {
    assert_eq!(
        common::eval("outer: { leaked = 1; break outer; class C {} } typeof leaked"),
        Ok("string:number".to_string())
    );
    assert_eq!(
        common::eval("outer: { leaked = 1; break outer; } typeof leaked"),
        Ok("string:number".to_string())
    );
}

#[test]
fn a_closure_made_in_a_non_declaring_block_captures_the_enclosing_binding() {
    assert_eq!(
        common::eval("var n = 1; var f; { f = function () { return n; }; } n = 2; f()"),
        Ok("number:2".to_string())
    );
    assert_eq!(
        common::eval(
            "var n = 0; var inc; var read; { inc = function () { n = n + 1; }; read = function () { return n; }; } inc(); inc(); read()"
        ),
        Ok("number:2".to_string())
    );
}

#[test]
fn a_try_body_without_declarations_shares_the_enclosing_scope_and_the_catch_still_binds() {
    assert_eq!(
        common::eval(
            "var a = 1; var f; try { a = 2; f = function () { return a; }; } catch (e) {} a = 3; f()"
        ),
        Ok("number:3".to_string())
    );
    assert_eq!(
        common::eval("var a = 1; try { let a = 2; } catch (e) {} a"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        common::eval("var out = 0; try { throw 7; } catch (e) { out = e; } out"),
        Ok("number:7".to_string())
    );
    assert_eq!(
        common::eval("try { throw 7; } catch (e) {} typeof e"),
        Ok("string:undefined".to_string())
    );
}

#[test]
fn an_implicit_global_is_a_global_whatever_scope_assigns_it() {
    let wrong = [
        "{ x = 1; } typeof x",
        "{ let y = 0; x = 1; } typeof x",
        "function f() { x = 1; } f(); typeof x",
        "if (true) { let y; x = 1; } typeof x",
    ]
    .iter()
    .filter_map(|program| {
        let answer = common::eval(program);
        (answer != Ok("string:number".to_string())).then(|| format!("{program} -> {answer:?}"))
    })
    .collect::<Vec<_>>();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

fn bail_of(env: &FrameRef<'static>, name: untwist_eval::names::Name) -> Option<Bail> {
    match read_binding(env, name, || Bail::unresolved_global("missing")) {
        Err(abrupt) => match abrupt.into_kind() {
            AbruptKind::Bail(bail) => Some(bail),
            AbruptKind::Throw(_) => None,
        },
        Ok(_) => None,
    }
}

#[test]
fn reading_a_binding_answers_what_lookup_answers_and_bails_with_the_callers_bail() {
    let mut names = Names::default();
    let x = names.intern("x");
    let missing = names.intern("missing");
    let root = root_env();
    declare(&root, x, Value::Number(1.0));
    let inner = child_env(&root);
    declare(&inner, x, Value::Number(2.0));
    let unbound = || Bail::unresolved_global("missing");
    assert!(matches!(read_binding(&inner, x, unbound), Ok(Value::Number(n)) if n == 2.0));
    assert!(matches!(read_binding(&root, x, unbound), Ok(Value::Number(n)) if n == 1.0));
    assert!(matches!(Frame::slot_by_name(&inner, missing), Ok(None)));
    assert_eq!(
        bail_of(&inner, missing),
        Some(Bail::unresolved_global("missing"))
    );

    let mut beyond_cap = root.clone();
    for _ in 0..SCOPE_CHAIN_CAP + 10 {
        beyond_cap = child_env(&beyond_cap);
    }
    assert_eq!(bail_of(&beyond_cap, x), Some(Bail::ChainTooDeep(&"scope")));
    assert!(matches!(
        Frame::slot_by_name(&beyond_cap, x),
        Err(ScopeChainTooDeep)
    ));
}

#[test]
fn an_immutable_binding_reports_the_write_and_keeps_its_value() {
    let mut names = Names::default();
    let f = names.intern("f");
    let named = root_env();
    Frame::declare_immutable(&named, f, Value::Number(1.0));
    let inner = child_env(&named);
    assert_eq!(
        assign(&inner, f, Value::Number(2.0)),
        Ok(Assigned::Immutable)
    );
    assert!(matches!(Frame::slot_by_name(&inner, f), Ok(Some(Value::Number(n))) if n == 1.0));
}
