use crate::depth_mix::{depth, seeded, Rng, SAMPLE_DEPTHS};

const NAMES: &[&str] = &[
    "as",
    "satisfies",
    "implements",
    "type",
    "declare",
    "interface",
    "namespace",
    "module",
    "abstract",
    "readonly",
    "keyof",
    "infer",
    "override",
    "accessor",
    "get",
    "set",
    "static",
    "async",
    "of",
    "let",
    "private",
    "\u{e9}",
    "\u{fc}ber",
    "$\u{e9}",
    "_\u{ea}",
    "\u{3c0}",
    "\x5cu0061s",
    "s\x5cu0061tisfies",
    "\x5cu{74}ype",
    "a\x5cu0073",
    "\u{e9}\x5cu0061",
    "class",
    "function",
    "if",
    "#p",
];

const DECORATORS: &[&str] = &[
    "@dec ",
    "@dec() ",
    "@a.b ",
    "@a.b() ",
    "@(x) ",
    "@dec(1, [2]) ",
    "@dec\n",
    "@a @b ",
    "@dec`t` ",
];

const MEMBERS: &[(&str, &str)] = &[
    ("", "(){}"),
    ("static ", "(){}"),
    ("async ", "(){}"),
    ("get ", "(){}"),
    ("set ", "(v){}"),
    ("accessor ", " = 1;"),
    ("static async ", "(){}"),
    ("async *", "(){}"),
    ("*", "(){}"),
    ("static ", " = 1;"),
    ("", " = 1;"),
    ("", ";"),
    ("static get ", "(){}"),
    ("", "\n"),
    ("", " = {}\n"),
    ("", " = () => {};"),
    ("static ", "\n"),
];

const PROPERTIES: &[(&str, &str)] = &[
    ("", ": 1"),
    ("", "(){}"),
    ("get ", "(){}"),
    ("set ", "(v){}"),
    ("async ", "(){}"),
    ("*", "(){}"),
];

const LABELS: &[&str] = &[
    "@: for (;;) { break @ }",
    "@: for (;;) { continue @ }",
    "@: { break @ }",
    "@: while (0) continue @;",
    "@: for (;;) break @\n",
];

const BINDINGS: &[&str] = &[
    "var @ = 1; @ = @ / 2 / 1;",
    "function f(@) { return @ }",
    "x = (@) => @;",
    "x = @ => @;",
    "x = async @ => @;",
    "try {} catch (@) {}",
    "for (const @ of []);",
    "let { @ } = { @: 1 };",
    "x.@ / 2; x?.@ / 2;",
    "import { @ as y } from 'm';",
    "export { x as @ };",
    "import * as @ from 'm';",
    "export * as @ from 'm';",
];

fn key(rng: &mut Rng) -> String {
    let name = rng.pick(NAMES);
    match rng.below(6) {
        0 => format!("[{name}]"),
        1 => format!("[ /*k*/ {name} ]"),
        _ => name.to_owned(),
    }
}

fn member(rng: &mut Rng) -> String {
    let mut text = String::new();
    for _ in 0..rng.below(3) {
        text.push_str(rng.pick(DECORATORS));
    }
    let (prefix, tail) = MEMBERS[rng.below(MEMBERS.len())];
    format!("{text}{prefix}{}{tail}", key(rng))
}

pub fn soup_member(seed: u64) -> String {
    member(&mut Rng::new(seed))
}

fn class(rng: &mut Rng) -> String {
    let members: Vec<String> = (0..1 + rng.below(3)).map(|_| member(rng)).collect();
    let body = members.join(" ");
    match rng.below(4) {
        0 => format!("x = class {{ {body} }};"),
        1 => format!("class C extends f(class {{ {body} }}) {{}}"),
        _ => format!("class C {{ {body} }}"),
    }
}

fn object(rng: &mut Rng) -> String {
    let properties: Vec<String> = (0..1 + rng.below(3))
        .map(|_| {
            let (prefix, tail) = PROPERTIES[rng.below(PROPERTIES.len())];
            format!("{prefix}{}{tail}", key(rng))
        })
        .collect();
    format!("x = {{ {} }};", properties.join(", "))
}

fn named(rng: &mut Rng, forms: &[&str]) -> String {
    let name = rng.pick(NAMES);
    rng.pick(forms).replace('@', name)
}

fn region(rng: &mut Rng, n: usize) -> String {
    match rng.below(4) {
        0 => format!("{}x{}", "[".repeat(n), "]".repeat(n)),
        1 => format!("{}1{}", "({a:".repeat(n), "})".repeat(n)),
        2 => format!("{}x{}", "`${".repeat(n), "}`".repeat(n)),
        _ => format!("{}x{}", "(".repeat(n), ")".repeat(n)),
    }
}

fn statement(rng: &mut Rng) -> (String, &'static str) {
    match rng.below(5) {
        0 => (named(rng, LABELS), "label"),
        1 => (object(rng), "object"),
        2 => (named(rng, BINDINGS), "binding"),
        _ => (class(rng), "class"),
    }
}

pub fn program(seed: u64, index: u64) -> (String, String) {
    let mut rng = seeded(seed, index);
    let mut text = String::new();
    let mut kinds = Vec::new();
    for _ in 0..1 + rng.below(3) {
        let (part, kind) = statement(&mut rng);
        text.push_str(&part);
        kinds.push(kind);
        text.push('\n');
        if rng.chance(50) {
            let n = depth(&mut rng, &SAMPLE_DEPTHS);
            let deep = region(&mut rng, n);
            text.push_str(&format!("/`/;\n{deep}\n;/`/\n"));
        }
    }
    let n = depth(&mut rng, &SAMPLE_DEPTHS);
    let deep = region(&mut rng, n);
    text.push_str(&format!("x = {deep};\n"));
    (text, format!("names {}", kinds.join(",")))
}
