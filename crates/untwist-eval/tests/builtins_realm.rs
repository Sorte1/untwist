mod common;

use common::eval;

#[test]
fn hostile_globals_are_bound_but_bail_when_called() {
    assert_eq!(eval("typeof eval"), Ok("string:function".to_string()));
    assert_eq!(
        eval("eval('1')"),
        Err("hostile global eval reached".to_string())
    );
    assert_eq!(eval("(function(){}).constructor"), Ok("native".to_string()));
    assert_eq!(
        eval("(function(){}).constructor('debugger')"),
        Err("hostile global Function reached".to_string())
    );
    assert_eq!(
        eval("new Date()"),
        Err("hostile global Date reached".to_string())
    );
    assert_eq!(
        eval("setTimeout(function(){})"),
        Err("hostile global setTimeout reached".to_string())
    );
}

#[test]
fn a_hostile_global_bail_is_not_catchable() {
    assert_eq!(
        eval("try { eval('1'); } catch (e) { 'caught' }"),
        Err("hostile global eval reached".to_string())
    );
}

#[test]
fn an_unbound_global_is_still_a_bail_not_undefined() {
    assert_eq!(
        eval("someRandomHostThing"),
        Err("unresolved global binding: someRandomHostThing".to_string())
    );
}

#[test]
fn the_standard_value_globals_are_present() {
    assert_eq!(eval("typeof undefined"), Ok("string:undefined".to_string()));
    assert_eq!(eval("NaN !== NaN"), Ok("boolean:true".to_string()));
    assert_eq!(eval("Infinity"), Ok("number:Infinity".to_string()));
    assert_eq!(eval("typeof globalThis"), Ok("string:object".to_string()));
}

#[test]
fn hostile_host_objects_are_objects_and_refuse_every_access() {
    assert_eq!(
        eval("typeof window"),
        Err("hostile global window reached".to_string())
    );
    assert_eq!(
        eval("typeof document"),
        Err("hostile global document reached".to_string())
    );
    assert_eq!(
        eval("typeof navigator + typeof process"),
        Ok("string:objectobject".to_string())
    );
    assert_eq!(
        eval("window['foo']"),
        Err("hostile global window reached".to_string())
    );
    assert_eq!(
        eval("window['foo'] = 1"),
        Err("hostile global window reached".to_string())
    );
    assert_eq!(
        eval("navigator['userAgent']"),
        Err("hostile global navigator reached".to_string())
    );
    assert_eq!(
        eval("new window()"),
        Err("hostile global window reached".to_string())
    );
}

#[test]
fn a_property_read_on_a_hostile_function_global_also_bails() {
    assert_eq!(
        eval("Function['prototype']"),
        Err("hostile global Function reached".to_string())
    );
}

#[test]
fn deleting_a_property_on_a_hostile_global_bails() {
    assert_eq!(
        eval("delete window['foo']"),
        Err("hostile global window reached".to_string())
    );
    assert_eq!(
        eval("delete process['env']"),
        Err("hostile global process reached".to_string())
    );
}

#[test]
fn both_global_bail_constructors_take_a_borrowed_or_an_owned_name() {
    use untwist_eval::bail::Bail;
    assert_eq!(
        Bail::unresolved_global("x"),
        Bail::unresolved_global(String::from("x"))
    );
    assert_eq!(
        Bail::hostile_global("window"),
        Bail::hostile_global(String::from("window"))
    );
    assert_eq!(
        Bail::hostile_global("window").to_string(),
        "hostile global window reached"
    );
    assert_eq!(
        Bail::unresolved_global("x").to_string(),
        "unresolved global binding: x"
    );
}

#[test]
fn install_consumes_the_globals_and_moves_every_one_into_the_env() {
    use untwist_eval::builtins::Realm;
    use untwist_eval::env::root_env;
    use untwist_eval::frame::Frame;
    use untwist_eval::names::Names;
    let (_realm, globals) = Realm::with_globals();
    let mut names = Names::default();
    assert!(globals.get("Math").is_some());
    let env = root_env();
    globals.install(&env, &mut names);
    let math = names.intern("Math");
    let undefined = names.intern("undefined");
    assert!(matches!(Frame::slot_by_name(&env, math), Ok(Some(_))));
    assert!(matches!(Frame::slot_by_name(&env, undefined), Ok(Some(_))));
}

#[test]
fn a_session_starts_with_every_global_already_in_its_one_root_env() {
    use untwist_eval::budget::Budget;
    use untwist_eval::frame::Frame;
    use untwist_eval::interp::Interp;
    use untwist_eval::names::STATIC_NAMES;
    let mut interp = Interp::new("", Budget::default());
    let env = interp.global_env();
    assert!(std::rc::Rc::ptr_eq(&env, &interp.global_env()));
    for text in STATIC_NAMES.iter().filter(|text| **text != "arguments") {
        let name = interp.name_of(text);
        assert!(
            matches!(Frame::slot_by_name(&env, name), Ok(Some(_))),
            "{text}"
        );
    }
    assert!(interp.names.is_empty());
}
