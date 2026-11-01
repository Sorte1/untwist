use super::pool::Rng;
use super::statements::pick_value;

const STRICT_PERCENT: u64 = 20;
const ESCAPE_PERCENT: u64 = 30;
const MAX_OPERATIONS: usize = 5;

struct Parameters {
    list: &'static str,
    names: &'static [&'static str],
    strict_safe: bool,
}

const fn parameters(
    list: &'static str,
    names: &'static [&'static str],
    strict_safe: bool,
) -> Parameters {
    Parameters {
        list,
        names,
        strict_safe,
    }
}

const PARAMETERS: &[Parameters] = &[
    parameters("p0", &["p0"], true),
    parameters("p0, p1", &["p0", "p1"], true),
    parameters("p0, p1, p2", &["p0", "p1", "p2"], true),
    parameters("p0, p0", &["p0"], false),
    parameters("p0, p1, p0", &["p0", "p1"], false),
    parameters("p0, p1 = 2", &["p0", "p1"], false),
    parameters("p0, ...p1", &["p0", "p1"], false),
    parameters("p0, arguments", &["p0"], false),
];

const INDICES: &[&str] = &[
    "0", "1", "2", "3", "'0'", "'1'", "'01'", "1.5", "-0", "'length'",
];

const OPERATIONS: &[&str] = &[
    "@P = @V;",
    "@P = @P - 1;",
    "@P++;",
    "arguments[@I] = @V;",
    "arguments[@I] += 1;",
    "log += '|' + @P;",
    "log += '|' + arguments[@I];",
    "log += '|' + typeof arguments[@I];",
    "log += '|' + (@I in arguments);",
    "delete arguments[@I];",
    "var c = arguments; c[@I] = @V;",
    "var c = arguments; @P = @V; log += '|' + c[@I];",
    "(function (o) { o[@I] = @V; })(arguments);",
    "(() => { @P = @V; })();",
    "(() => { arguments[@I] = @V; })();",
    "log += '|' + (() => arguments[@I])();",
    "for (var x of arguments) { log += '|' + x; @P = @V; }",
    "for (var x in arguments) { log += '|' + x; }",
    "log += '|' + JSON.stringify(arguments);",
    "log += '|' + Object.keys(arguments);",
    "log += '|' + (function () { return [].join.call(arguments, '/'); }).apply(null, arguments);",
    "log += '|' + (function (a, b) { return a + '/' + b; }).apply(null, arguments);",
    "arguments.length = @A;",
    "log += '|' + arguments.length;",
    "function @P() { return 'h'; }",
];

fn one<'s>(rng: &mut Rng, list: &[&'s str]) -> &'s str {
    list[rng.below(list.len())]
}

fn parameter(rng: &mut Rng, parameters: &Parameters) -> &'static str {
    parameters.names[rng.below(parameters.names.len())]
}

fn operation(rng: &mut Rng, parameters: &Parameters) -> String {
    let template = one(rng, OPERATIONS);
    template
        .replace("@P", parameter(rng, parameters))
        .replace("@I", one(rng, INDICES))
        .replace("@A", &rng.below(4).to_string())
        .replace("@V", pick_value(rng))
}

fn passed(rng: &mut Rng) -> String {
    let count = rng.below(4);
    (0..count)
        .map(|_| pick_value(rng).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn reads(parameters: &Parameters) -> String {
    let mut out: Vec<String> = parameters
        .names
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    out.extend((0..3).map(|index| format!("arguments[{index}]")));
    out.push("arguments.length".to_string());
    out.join(", ")
}

fn strict_directive(rng: &mut Rng, parameters: &Parameters) -> &'static str {
    if parameters.strict_safe && rng.chance(STRICT_PERCENT) {
        "'use strict'; "
    } else {
        ""
    }
}

pub fn aliasing_case(rng: &mut Rng) -> (String, String) {
    let parameters = &PARAMETERS[rng.below(PARAMETERS.len())];
    let outer = strict_directive(rng, parameters);
    let directive = strict_directive(rng, parameters);
    let (label, body) = aliasing_body(rng, parameters, directive);
    let label = format!("{label}{}", outer.trim());
    (
        label,
        format!("(function () {{ {outer}var log = ''; {body} }})()"),
    )
}

fn aliasing_body(rng: &mut Rng, parameters: &Parameters, directive: &str) -> (String, String) {
    let list = parameters.list;
    let operations: Vec<String> = (0..1 + rng.below(MAX_OPERATIONS))
        .map(|_| operation(rng, parameters))
        .collect();
    let operations = operations.join(" ");
    let args = passed(rng);
    let label = format!("stmt arguments ({list}){}", directive.trim());
    if !rng.chance(ESCAPE_PERCENT) {
        let reads = reads(parameters);
        let body = format!(
            "function f({list}) {{ {directive}{operations} return [{reads}]; }} return [f({args}), log];"
        );
        return (label, body);
    }
    let name = parameter(rng, parameters);
    let index = one(rng, INDICES);
    let value = pick_value(rng);
    if rng.chance(50) {
        let body = format!(
            "function f({list}) {{ {directive}{operations} return arguments; }} var t = f({args}); log += '|' + t[{index}]; t[{index}] = {value}; return [t[0], t[1], t[2], t.length, log];"
        );
        return (format!("{label} returned"), body);
    }
    let body = format!(
        "function f({list}) {{ {directive}{operations} return [arguments, function () {{ return {name}; }}, function (v) {{ {name} = v; }}]; }} var t = f({args}); t[2]({value}); log += '|' + t[0][{index}]; t[0][{index}] = 'w'; return [t[1](), t[0][0], t[0][1], t[0].length, log];"
    );
    (format!("{label} escaped"), body)
}
