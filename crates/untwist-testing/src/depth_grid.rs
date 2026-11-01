use crate::depth_soup::Soup;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        for _ in 0..4 {
            rng.draw();
        }
        rng
    }

    pub fn draw(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.draw() % n.max(1) as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const WRAPPERS: &[&str] = &[
    "@",
    "function* g(){ @ }",
    "async function f(){ @ }",
    "async function* f(){ @ }",
    "function f(){ @ }",
    "class A { m(){ @ } }",
    "class A { *m(){ @ } }",
    "class A { async m(){ @ } }",
    "class A { static { @ } }",
    "x = { *m(){ @ } }",
    "x = async () => { @ }",
    "function* g(){ function h(){ @ } }",
    "async function f(){ function h(){ @ } }",
    "function* g(){ async function h(){ @ } }",
    "async function f(){ () => { @ } }",
    "function* g(){ () => { @ } }",
    "for await (x of y) { @ }",
    "async function f(){ for await (x of y) ; @ }",
    "if (a) { @ }",
    "switch (a) { case 1: @ }",
    "do { @ } while (0)",
    "label: { @ }",
    "with (a) { @ }",
    "x = function(){ @ }",
    "\"use strict\"; @",
    "export default function(){ @ }",
    "<a>{ (() => { @ })() }</a>",
    "`${ (() => { @ })() }`",
];

const PREFIXES: &[&str] = &[
    "",
    "a",
    "a\n",
    "a ",
    "a++\n",
    "a\n++",
    "++\n",
    "a--",
    "x = y\n",
    "yield",
    "yield\n",
    "-yield",
    "!yield",
    "typeof yield",
    "x = yield",
    "(yield)",
    "yield*",
    "yield\n*",
    "await",
    "await\n",
    "-await",
    "!await",
    "(await)",
    "x = await",
    "await\n*",
    "await await",
    "yield yield",
    "let",
    "let\n",
    "var let\n",
    "let let\n",
    "let x\n",
    "let x =",
    "const x = 1\n",
    "var a, b\n",
    "if (a)",
    "if (a) b;\nelse",
    "while (a)",
    "for (;;)",
    "for (x of y)",
    "for (x in y)",
    "for (let x of y)",
    "for (const x in y)",
    "for await (x of y)",
    "with (a)",
    "do",
    "do ;",
    "do x; while(0)",
    "do ; while(0)",
    "do {} while(0)",
    "do do ; while(0) while(0)",
    "do while(0); while(0)",
    "do if (a) ; while(0)",
    "do x\nwhile(0)",
    "function f(){}",
    "function f(){}\n",
    "x = function(){}\n",
    "x = function(){}",
    "class A {}",
    "class A {}\n",
    "x = class {}\n",
    "class A extends B {}",
    "({})",
    "{}",
    "{}\n",
    "}",
    ")",
    "]",
    "x = {a: 1}\n",
    "x = {}",
    "return",
    "return\n",
    "throw",
    "of",
    "for (x of",
    "for (var of",
    "for (of of",
    "for (let of",
    "for (async of",
    "for (let\nof",
    "a => {}\n",
    "a => {}",
    "a => ({})\n",
    "x = a => {}\n",
    "(a) => b\n",
    "(a) => b",
    "async a => a\n",
    "async () => {}\n",
    "async\n() => {}",
    "async\nfunction f(){}",
    "async",
    "async\n",
    "case 1:",
    "case b ? c : d:",
    "case b ? {c} : d:",
    "default:",
    "a ? b : c\n",
    "a ? b :",
    "a ?",
    "a ?.5 :",
    "a ? .5 :",
    "`${a}`\n",
    "`${a}`",
    "tag`x`\n",
    "x\n",
    "1\n",
    "'s'\n",
    "/re/\n",
    "/re/",
    "a.if\n",
    "a?.if\n",
    "a. if\n",
    "a.\nif\n",
    "a.yield\n",
    "a\n.yield",
    "a.await\n",
    "a.of\n",
    "a.let\n",
    "a.async\n",
    "a.function\n",
    "a.class\n",
    "x = { if: 1 }\n",
    "x = { yield }\n",
    "x = { await }\n",
    "#a in x",
    "this\n",
    "super\n",
    "null\n",
    "true\n",
    "new.target",
    "new.target\n",
    "import.meta",
    "import.meta\n",
    "import('a')\n",
    "export default",
    "export default function(){}",
    "export default class {}",
    "export default async function(){}",
    "export { a as default }\n",
    "import a from 'b'\n",
    "get\n",
    "static\n",
    "set\n",
    "of\n",
    "as\n",
    "from\n",
    "target\n",
    "x = { get a(){} }\n",
    "try {} catch {}",
    "try {} finally {}",
    "debugger\n",
    "new",
    "delete",
    "typeof",
    "void",
    "x in",
    "x instanceof",
    "!",
    "a?.(",
    "a?.[",
    "1.",
    "1.e",
    "0x",
    "1_",
    "1n",
    "1.5\n",
    ".5",
    "a\\u0020",
    "\\u0061wait",
    "yi\\u0065ld",
    "l\\u0065t\n",
    "\\u{79}ield",
    "x = a\n?.b",
    "label:",
    "a: { break a; }",
    "<a/>",
    "<a></a>",
    "<a></a>\n",
    "<></>",
    "x = <a/>\n",
    "a <b>",
    "if (a) <b></b>",
    "a <!--",
    "a\n<!--",
    "x\n-->",
    "-->",
    "/* */ -->",
    "/*\n*/ -->",
    "\u{2028}-->",
    "x\u{2028}-->",
    "x\u{2029}",
    "x\r",
    "y\u{2028}",
    "a /* \u{2028} */",
    "\u{feff}#!",
    "#!",
    "a\u{a0}",
    "a\u{feff}",
    "x = `\n`\n",
    "x = '\\\n'\n",
    "x = \"\u{2028}\"",
    "'\u{2028}'",
    "\"\\\n\"",
    "f()\n",
    "f()",
    "a[0]\n",
    "a[0]",
    "(a)\n",
    "(a)",
    "a\n(b)\n",
    "var x\n(",
    "let x\n(",
    "let [a] = b\n",
    "let {a} = b\n",
    "var a = b\n",
    "x = 1,\n",
    "async function f(){}\n",
    "function* g(){}\n",
    "x = function*(){}\n",
    "class A { static }\n",
    "class A { get }\n",
    "class A { x = 1\n }",
    "class A { [a] }",
    "x = { a, b }\n",
    "x = {...a}\n",
    "[a] = b\n",
    "({a} = b)\n",
    "a = b = c\n",
    "a ? (b) : (c)\n",
    "if (a) b\nelse c\n",
    "if (a) {} else {}",
    "if (a) {}",
    "while (a) {}",
    "for (;;) {}",
    "try {}\ncatch (e) {}",
    "switch (a) {}",
    "label: for (;;) break label\n",
    "continue\n",
    "break\n",
    "import x from 'y';",
    "export * from 'y'\n",
    "export const a = 1\n",
    "export let",
    "export let\n",
    "x = async\n",
    "async /* */",
    "async function f(){ await",
    "function g(){ yield",
    "function* g(){ yield",
    "new yield",
    "new await",
    "new\nyield",
    "class A extends yield",
    "class A extends await",
    "x = class extends yield",
    "a ** yield",
    "a + yield",
    "a ? yield",
    "a ? b : yield",
    "a || yield",
    "a, yield",
    "(a, yield)",
    "[yield",
    "...yield",
    "`${yield",
    "a = yield",
    "a += yield",
    "f(yield",
    "a[yield",
    "a ?? await",
    "void await",
    "delete yield",
    "typeof await",
    "++yield",
    "yield++",
    "await++",
    "a\n++yield",
    "-yield\n",
    "x = -yield",
    "a.b = yield",
    "a ? b : -yield",
    "new yield.a",
    "new await.a",
    "yield.a",
    "await.a",
    "yield?.a",
    "a in yield",
    "!(yield)",
    "async () => yield",
    "() => yield",
    "() => await",
    "async () => -await",
    "(a = yield) =>",
    "(a = await) =>",
    "async (a = await) =>",
    "function* g(a = yield",
    "x = { [yield",
    "class { [yield",
    "class { x = yield",
    "class { static x = await",
    "x = { m(a = yield",
    "label: yield",
    "yield: ",
    "await: ",
    "for (yield of",
    "for (await of",
    "for (yield in",
    "for (;yield;",
    "if (yield)",
    "while (await)",
    "switch (yield) {}",
    "return yield",
    "throw yield",
    "export default yield",
    "export default await",
    "import(yield",
    "super(yield",
    "a?.(yield",
    "tag`${yield",
    "<a {...yield",
    "<a b={yield",
    "<a>{yield",
    "x = <a>{-yield",
];

const SLOTS: &[&str] = &[
    " /@/ ",
    "\n/@/\n",
    "/@/",
    " /`/;\n@\n;/`/ ",
    "\n/`/;\n@\n;/`/\n",
    " <a>\n@\n</a> ",
    "\n<a>\n@\n</a>\n",
    " <a b=\"\n@\n\"/> ",
    " <a>{`\n@\n`}</a> ",
    " `\n@\n` ",
    " {} /@/ ",
    " {}\n/`/;\n@\n;/`/ ",
    " ( /@/ ) ",
    " ` /@/ ` ",
    " ( @ ) ",
    " @ ",
    "\n@\n",
    " /[/]/; `\n@\n` ",
    " /'/; \"\n@\n\" ",
    " /x/ `\n@\n` ",
    " / x / `\n@\n` ",
    " <!--`\n@\n//` ",
    "\n--> `\n@\n// `\n",
    " < a > `\n@\n` ",
    " < /`/ > @ ",
    " /*/ `\n@\n` */ ",
    "\n{} /@/ ",
];

const REGIONS: &[&str] = &[
    "paren", "open", "bracket", "bang", "block", "objopen", "cond", "arrow", "call", "assign",
    "tmpl",
];

const GRID_REGIONS: &[&str] = &["open", "paren", "bang", "bracket"];

const TOKENS: &[&str] = &[
    "a",
    "b",
    "x",
    "1",
    "'s'",
    "\"t\"",
    "`",
    "${",
    "}",
    "{",
    "(",
    ")",
    "[",
    "]",
    ";",
    ",",
    "\n",
    " ",
    "/",
    "/",
    "/",
    "/=",
    "*",
    "+",
    "++",
    "--",
    "-",
    "!",
    "?",
    ":",
    "?.",
    ".",
    "=>",
    "=",
    "==",
    "<",
    ">",
    "</",
    "/>",
    "<!--",
    "-->",
    "//",
    "/*",
    "*/",
    "#!",
    "#a",
    "\\u0061",
    "yield",
    "await",
    "async",
    "function",
    "function*",
    "class",
    "let",
    "var",
    "const",
    "of",
    "in",
    "if",
    "else",
    "for",
    "while",
    "do",
    "return",
    "new",
    "typeof",
    "case",
    "default",
    "switch",
    "static",
    "get",
    "import",
    "export",
    "\u{2028}",
    "\r\n",
    "'",
    "\"",
    "...",
    "@",
    "\\",
    "0x",
    ".5",
    "e",
    "this",
    "super",
    "<a>",
    "</a>",
    "<>",
    "</>",
    "&&",
    "||",
    "??",
    "**",
    "in ",
    "instanceof",
    "void",
    "delete",
    "\\u{",
    "break",
    "continue",
    "debugger",
];

pub const GRID_DEPTH: usize = 2_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Grid,
    GridRand,
    Soup,
    Soup2,
    Rand,
    Sweep,
}

impl Kind {
    pub fn from_name(name: &str) -> Option<Kind> {
        Some(match name {
            "grid" => Kind::Grid,
            "gridrand" => Kind::GridRand,
            "soup" => Kind::Soup,
            "soup2" => Kind::Soup2,
            "rand" => Kind::Rand,
            "sweep" => Kind::Sweep,
            _ => return None,
        })
    }
}

pub fn grid_len() -> u64 {
    (WRAPPERS.len() * PREFIXES.len() * SLOTS.len() * GRID_REGIONS.len()) as u64
}

pub fn region(kind: &str, n: usize) -> String {
    match kind {
        "paren" => format!("{}x{}", "(".repeat(n), ")".repeat(n)),
        "open" => "(".repeat(n),
        "bracket" => "[".repeat(n),
        "bang" => format!("{}x", "!".repeat(n)),
        "block" => "{".repeat(n),
        "objopen" => "({a:".repeat(n),
        "cond" => format!("{}x", "a?b:".repeat(n)),
        "arrow" => format!("{}x", "()=>".repeat(n)),
        "call" => "f(".repeat(n),
        "assign" => format!("{}0", "x=".repeat(n)),
        _ => "`${".repeat(n),
    }
}

fn grid(index: u64, depth: usize) -> (String, String) {
    let mut rest = usize::try_from(index % grid_len()).unwrap_or(0);
    let mut take = |len: usize| {
        let value = rest % len;
        rest /= len;
        value
    };
    let w = take(WRAPPERS.len());
    let p = take(PREFIXES.len());
    let s = take(SLOTS.len());
    let kind = GRID_REGIONS[take(GRID_REGIONS.len())];
    let body = format!(
        "{}{}",
        PREFIXES[p],
        SLOTS[s].replace('@', &region(kind, depth))
    );
    let tag = format!("grid w{w} p{p} s{s} {kind}");
    (WRAPPERS[w].replacen('@', &body, 1), tag)
}

fn splice(base: &str, rng: &mut Rng, deep: &str) -> String {
    let mut at = rng.below(base.len() + 1);
    while !base.is_char_boundary(at) {
        at -= 1;
    }
    format!("{}{}{}", &base[..at], deep, &base[at..])
}

fn deep(rng: &mut Rng, sizes: &[usize]) -> (String, String) {
    let kind = rng.pick(REGIONS);
    let n = sizes[rng.below(sizes.len())];
    (region(kind, n), format!("{kind}{n}"))
}

fn soup(rng: &mut Rng, twice: bool, sizes: &[usize]) -> (String, String) {
    let base = Soup::new(rng.draw()).program();
    let (first, tag) = deep(rng, sizes);
    let once = splice(&base, rng, &first);
    if !twice {
        return (once, format!("soup {tag}"));
    }
    let (second, tag2) = deep(rng, &sizes[..1]);
    (splice(&once, rng, &second), format!("soup2 {tag} {tag2}"))
}

fn rand(rng: &mut Rng, sizes: &[usize]) -> (String, String) {
    let mut out = String::new();
    let count = 2 + rng.below(25);
    let at = rng.below(count + 1);
    let (region, tag) = deep(rng, sizes);
    for k in 0..=count {
        if k == at {
            out.push_str(&region);
        }
        if k < count {
            out.push_str(rng.pick(TOKENS));
            if rng.below(3) == 0 {
                out.push(' ');
            }
        }
    }
    (out, format!("rand {tag}"))
}

fn gridrand(rng: &mut Rng, sizes: &[usize]) -> (String, String) {
    let wrapper = rng.pick(WRAPPERS);
    let first = rng.pick(PREFIXES);
    let second = rng.pick(PREFIXES);
    let slot = rng.pick(SLOTS);
    let (region, tag) = deep(rng, sizes);
    let gap = if rng.below(2) == 0 { "\n" } else { " " };
    let body = format!("{first}{gap}{second}{}", slot.replace('@', &region));
    (wrapper.replacen('@', &body, 1), format!("gridrand {tag}"))
}

fn sweep(rng: &mut Rng) -> (String, String) {
    let sizes = [1_000, 10_000, 50_000, 100_000, 200_000];
    let base = if rng.below(2) == 0 {
        Soup::new(rng.draw()).program()
    } else {
        format!("{}{}", rng.pick(PREFIXES), rng.pick(SLOTS))
    };
    let (region, tag) = deep(rng, &sizes);
    let text = if base.contains('@') {
        base.replacen('@', &region, 1)
    } else {
        splice(&base, rng, &region)
    };
    let wrapper = rng.pick(WRAPPERS);
    (wrapper.replacen('@', &text, 1), format!("sweep {tag}"))
}

pub fn program(kind: Kind, seed: u64, index: u64) -> (String, String) {
    let mut rng = Rng::new(seed ^ index.wrapping_mul(0x1234_5678_9abc_def1));
    match kind {
        Kind::Grid => grid(index, GRID_DEPTH),
        Kind::GridRand => gridrand(&mut rng, &[1_000, 5_000]),
        Kind::Soup => soup(&mut rng, false, &[1_000, 3_000]),
        Kind::Soup2 => soup(&mut rng, true, &[1_000, 3_000]),
        Kind::Rand => rand(&mut rng, &[1_000, 3_000]),
        Kind::Sweep => sweep(&mut rng),
    }
}

pub const SAMPLE_DEPTHS: [usize; 2] = [400, 800];

pub fn sampled(seed: u64, index: u64) -> (String, String) {
    let mut rng = Rng::new(seed ^ index.wrapping_mul(0x0f1e_2d3c_4b5a_6978));
    let sizes = &SAMPLE_DEPTHS;
    match rng.below(10) {
        0..=4 => grid(rng.draw(), sizes[0]),
        5 | 6 => gridrand(&mut rng, sizes),
        7 => soup(&mut rng, false, sizes),
        8 => soup(&mut rng, true, sizes),
        _ => rand(&mut rng, sizes),
    }
}
