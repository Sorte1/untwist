pub enum Source {
    Script(String),
    Session(String),
}

pub struct Shape {
    pub name: &'static str,
    pub width: usize,
    pub calls: u64,
}

const fn shape(name: &'static str, width: usize, calls: u64) -> Shape {
    Shape { name, width, calls }
}

const WIDE: usize = 50_000;
const SWEPT: usize = 32_000;

pub const SHAPES: &[Shape] = &[
    shape("lets_body", WIDE, 1),
    shape("lets_block", WIDE, 1),
    shape("defaults", WIDE, 1),
    shape("lets_catch", SWEPT, 1),
    shape("lets_switch", SWEPT, 1),
    shape("top_lets", SWEPT, 0),
    shape("let_declarators", SWEPT, 1),
    shape("for_let_head", SWEPT, 1),
    shape("params", SWEPT, 1),
    shape("arguments", SWEPT, 1),
    shape("vars", SWEPT, 1),
    shape("var_declarators", SWEPT, 1),
    shape("functions_body", SWEPT, 3),
    shape("functions_block", SWEPT, 3),
    shape("closures", SWEPT, 1),
    shape("let_closures", SWEPT, 3),
    shape("object", SWEPT, 1),
    shape("object_computed", SWEPT, 1),
    shape("array", SWEPT, 1),
    shape("literal_array", SWEPT, 1),
    shape("template", SWEPT, 1),
    shape("sequence", SWEPT, 1),
    shape("switch_cases", SWEPT, 1),
    shape("switch_lets", SWEPT, 1),
    shape("statements", SWEPT, 1),
    shape("top_statements", SWEPT, 0),
    shape("for_in_keys", SWEPT, 1),
    shape("labels", SWEPT, 1),
    shape("catches", SWEPT, 1),
    shape("blocks", SWEPT, 1),
    shape("prelude_lets", SWEPT, 9),
    shape("prelude_vars", SWEPT, 9),
    shape("prelude_functions", SWEPT, 9),
];

fn list(width: usize, item: impl Fn(usize) -> String, separator: &str) -> String {
    (0..width).map(item).collect::<Vec<_>>().join(separator)
}

fn in_function(body: String) -> Source {
    Source::Script(format!("var r = (function () {{ {body} }})(); r;"))
}

fn lets(width: usize) -> String {
    list(width, |i| format!("let v{i} = {i};"), " ")
}

fn declarations(name: &str, width: usize) -> Option<Source> {
    let last = width - 1;
    let read = format!("return v0 + v{last};");
    Some(match name {
        "lets_body" => in_function(format!("{} {read}", lets(width))),
        "lets_block" => in_function(format!("{{ {} {read} }}", lets(width))),
        "lets_catch" => in_function(format!(
            "try {{ throw 0; }} catch (e) {{ {} {read} }}",
            lets(width)
        )),
        "lets_switch" => in_function(format!("switch (1) {{ case 1: {} {read} }}", lets(width))),
        "top_lets" => Source::Script(format!("{} v0 + v{last};", lets(width))),
        "let_declarators" => in_function(format!(
            "let {}; {read}",
            list(width, |i| format!("v{i} = {i}"), ", ")
        )),
        "for_let_head" => in_function(format!(
            "for (let {}; v0 < 1; v0++) {{ }} return 1;",
            list(width, |i| format!("v{i} = 0"), ", ")
        )),
        "vars" => in_function(format!(
            "{} {read}",
            list(width, |i| format!("var v{i} = {i};"), " ")
        )),
        "var_declarators" => in_function(format!(
            "var {}; {read}",
            list(width, |i| format!("v{i} = {i}"), ", ")
        )),
        _ => return None,
    })
}

fn functions(name: &str, width: usize) -> Option<Source> {
    let last = width - 1;
    let parameters = |item: fn(usize) -> String| list(width, item, ", ");
    let declared = list(width, |i| format!("function f{i}() {{ return {i}; }}"), " ");
    let called = format!("return f0() + f{last}();");
    Some(match name {
        "defaults" => Source::Script(format!(
            "var r = (function ({}) {{ return a0 + a{last}; }})(); r;",
            parameters(|i| format!("a{i} = {i}"))
        )),
        "params" => Source::Script(format!(
            "var r = (function ({}) {{ return a0 + a{last}; }})(1, 2); r;",
            parameters(|i| format!("a{i}"))
        )),
        "arguments" => Source::Script(format!(
            "var x = 1; var r = (function () {{ return arguments.length; }})({}); r;",
            list(width, |_| "x".to_string(), ", ")
        )),
        "functions_body" => in_function(format!("{declared} {called}")),
        "functions_block" => in_function(format!("'use strict'; {{ {declared} {called} }}")),
        "closures" => in_function(format!(
            "var x = 1, fs = []; {} return fs.length;",
            list(width, |_| "fs.push(() => x);".to_string(), " ")
        )),
        "let_closures" => in_function(format!(
            "{} return v0() + v{last}();",
            list(width, |i| format!("let v{i} = () => {i};"), " ")
        )),
        _ => return None,
    })
}

fn literals(name: &str, width: usize) -> Option<Source> {
    let last = width - 1;
    let properties = list(width, |i| format!("p{i}: {i}"), ", ");
    Some(match name {
        "object" => in_function(format!(
            "var o = {{{properties}}}; return o.p0 + o.p{last};"
        )),
        "object_computed" => in_function(format!(
            "var x = 1; var o = {{{}}}; return o.p0 + o.p{last};",
            list(width, |i| format!("['p' + {i}]: x"), ", ")
        )),
        "array" => in_function(format!(
            "var x = 1; var a = [{}]; return a.length;",
            list(width, |_| "x".to_string(), ", ")
        )),
        "literal_array" => in_function(format!(
            "var a = [{}]; return a.length;",
            list(width, |i| format!("{i}"), ", ")
        )),
        "template" => in_function(format!(
            "var x = 1; var t = `{}`; return t.length;",
            list(width, |_| "${x}".to_string(), "-")
        )),
        "sequence" => in_function(format!(
            "var x = 1; return ({});",
            list(width, |_| "x".to_string(), ", ")
        )),
        "for_in_keys" => in_function(format!(
            "var o = {{{properties}}}, c = 0; for (var k in o) c++; return c;"
        )),
        _ => return None,
    })
}

fn statements(name: &str, width: usize) -> Option<Source> {
    let last = width - 1;
    let repeated = |text: &str| list(width, |_| text.to_string(), " ");
    Some(match name {
        "switch_cases" => in_function(format!(
            "var r = 0; switch ({last}) {{ {} }} return r;",
            list(width, |i| format!("case {i}: r = {i}; break;"), " ")
        )),
        "switch_lets" => in_function(format!(
            "switch ({last}) {{ {} }} return 0;",
            list(width, |i| format!("case {i}: let v{i} = {i}; break;"), " ")
        )),
        "statements" => in_function(format!("var x = 0; {} return x;", repeated("x = x + 1;"))),
        "top_statements" => Source::Script(format!("var x = 0; {} x;", repeated("x = x + 1;"))),
        "labels" => in_function(format!(
            "{} return 1;",
            list(width, |i| format!("l{i}: {{ break l{i}; }}"), " ")
        )),
        "catches" => in_function(format!(
            "var c = 0; {} return c;",
            repeated("try { throw 1; } catch (e) { c += e; }")
        )),
        "blocks" => in_function(format!(
            "var c = 0; {} return c;",
            list(width, |i| format!("{{ let b = {i}; c += b; }}"), " ")
        )),
        _ => return None,
    })
}

pub const ARRAY: &str = "function a() { var h = ['4', '17', '1', '2', '3', '5a', '9']; a = function () { return h; }; return a(); }";
pub const DECODER: &str =
    "function b(i, k) { i = i - 0x10; var e = a(); return e[i] + (k === undefined ? '' : '' + k); }";

fn prelude(name: &str, width: usize) -> Option<Source> {
    let declared = match name {
        "prelude_lets" => list(width, |i| format!("const p{i} = {i};"), " "),
        "prelude_vars" => list(width, |i| format!("var p{i} = {i};"), " "),
        "prelude_functions" => list(width, |i| format!("function p{i}() {{ return {i}; }}"), " "),
        _ => return None,
    };
    let rotator = format!(
        "(function (c, d) {{ const e = c(); const g = b; {declared} while (!![]) {{ try {{ const f = parseInt(g(0x10)); if (f === d) break; else e['push'](e['shift']()); }} catch (h) {{ e['push'](e['shift']()); }} }} }})(a, 0x4);"
    );
    Some(Source::Session(format!(
        "{ARRAY}\n{DECODER}\n{rotator}\nb(0x10);\nb(0x12, 'k');\n"
    )))
}

pub fn source(name: &str, width: usize) -> Option<Source> {
    declarations(name, width)
        .or_else(|| functions(name, width))
        .or_else(|| literals(name, width))
        .or_else(|| statements(name, width))
        .or_else(|| prelude(name, width))
}
