use std::path::PathBuf;
use std::sync::OnceLock;

pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        let mut rng = Rng(seed ^ 0xD1B5_4A32_D192_ED03);
        for _ in 0..8 {
            rng.next();
        }
        rng
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    pub(crate) fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    pub(crate) fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

const DOUBTFUL: &[&str] = &[
    "yield",
    "-yield",
    "yield\n",
    "x = yield",
    "(yield)",
    "[yield]",
    "{a: yield}",
    "yield of",
    "yield yield",
    "a ? yield : yield",
    "yield*",
    "void yield",
    "delete yield",
    "yield in a",
    "await",
    "await\n",
    "new await",
    "typeof await",
    "await of",
    "await await",
    "await (a)",
    "x = await",
    "-await",
    "await\n(a)",
    "async () => await",
    "for (await of",
    "await.x",
    "\\u0061wait",
    "\\u{79}ield",
    "aw\\u0061it",
    "yi\\u{065}ld",
    "l\\u0065t",
    "\\u006ff",
    "\\u0069n",
    "n\\u0065w",
    "\\u0074ypeof",
    "\\u0062reak",
    "d\\u0065bugger",
    "r\\u0065turn",
    "\\u0066unction",
    "\\u0063lass",
    "\\u{0000000061}sync",
    "\\u0069f (a)",
    "\\u0065lse",
    "\\u0069mport",
    "\\u0065xport",
    "\\u0079ield\n",
    "\\u0061wait\n",
    "@a",
    "@a.b",
    "@(a)",
    "@a()",
    "@",
    "@\n",
    "@a\nclass {}",
    "@a class B {}",
    "@ /x/",
    "break",
    "continue",
    "break l",
    "continue\nl",
    "debugger",
    "break /*\n*/",
    "break // c\n",
    "break\u{2028}",
    "debugger\r",
    "continue /*x*/ l",
    "break\r\nl",
    "import 'a'",
    "import a from 'a'",
    "import {a as b} from \"a\"",
    "import * as a from 'a'",
    "import a, {b} from 'a'",
    "export {a}",
    "export {a as default}",
    "export * from 'a'",
    "export * as b from 'a'",
    "import a from 'a' with {type:'json'}",
    "import a from 'a' assert {type:'json'}",
    "import defer * as a from 'a'",
    "import source a from 'a'",
    "export {a} from 'a' with {type:'x'}",
    "import.meta",
    "import('a')",
    "import.meta.url",
    "export default {}",
    "export default function(){}",
    "export default class{}",
    "export default async function(){}",
    "export default a",
    "export var a",
    "export let {a} = b",
    "export const [c] = d",
    "export function f(){}",
    "export class K {}",
    "export {}",
    "import {} from 'a'",
    "import a from\n'a'",
    "let {a}",
    "let [a]",
    "const {a} = b",
    "var {a}",
    "var [a] = [b]",
    "let {a} = {}",
    "for (let {a} of b)",
    "using a = b",
    "await using a = b",
    "let\n{a}",
    "var {a: {b}}",
    "for (x of\n{}",
    "for (let of\n",
    "for (async of",
    "for (x of\n[]",
    "for await (x of\n{}",
    "for (yield of",
    "for (let of of",
    "for (x of\n{} /",
];

const GRAMMAR: &[&str] = &[
    "class A { x = 1; static y; #z = 2; static #w; }",
    "class A { get x(){} set x(v){} }",
    "class A { static { } }",
    "class A { static async *m(){} }",
    "class A { #p(){} m(){ return #p in this } }",
    "class A { 'constructor'(){} }",
    "class A { static\nx }",
    "class A { get\nx(){} }",
    "class A { async\nm(){} }",
    "class A { accessor x }",
    "class A { [a] = b }",
    "class A { static = 1 }",
    "class A { get = 1; set = 2; static async = 3 }",
    "x = class { #a; get #b(){} }",
    "l: for(;;) break l",
    "a: b: c",
    "l:\n{}",
    "yield: 1",
    "await: 1",
    "let: 1",
    "async: 1",
    "of: 1",
    "get: 1",
    "static: 1",
    "async x => x",
    "async (x) => x",
    "async () => {}",
    "async\nx => x",
    "async\nfunction f(){}",
    "x = async function(){}",
    "({ async m(){} })",
    "({ async *m(){} })",
    "({ get a(){}, set a(v){} })",
    "({ get [a](){} })",
    "({ async, get, set, static })",
    "a?.b",
    "a?.(b)",
    "a?.[b]",
    "a?.5:1",
    "a ?.5 : b",
    "a?.b`c`",
    "new a?.b",
    "a?.\nb",
    "import('a')",
    "import.meta",
    "new.target",
    "function f(){ new.target }",
    "import(\n'a')",
    "1_000",
    "0x1F",
    "0b10",
    "0o7",
    "1e5",
    "1e-5",
    "1.5e+5",
    ".5",
    "5.",
    "1..x",
    "1.5.x",
    "1n",
    "0x1n",
    "08",
    "09.5",
    "1_0n",
    "0.0e0",
    "1e",
    "1_",
    "0x",
    "5..toString()",
    "'\\''",
    "\"\\\"\"",
    "'\\\n'",
    "'\\\r\n'",
    "'\u{2028}'",
    "'\u{2029}'",
    "'\\u{1F600}'",
    "'\\x41'",
    "'\\0'",
    "`\\``",
    "`${'`'}`",
    "`\\${`",
    "/\\//",
    "/[/]/",
    "/[\\]/]/",
    "/a/gimsuyd",
    "/\\u{1}/u",
    "/[`]/",
    "/=/",
    "/a/v",
    "/(?<n>a)\\k<n>/",
    "#!/usr/bin/env node\n",
    "\u{feff}",
    "<!-- c\n",
    "--> c\n",
    "\n--> c\n",
    "/*\n*/-->c\n",
    "\r",
    "\r\n",
    "\n",
    "\u{2028}",
    "\u{2029}",
    "\u{a0}",
    "\u{feff}",
    "\t",
    "\u{b}",
    "\u{c}",
    "using x = y",
    "await using x = y",
    "using\nx",
    "for (using x of y);",
    "using[a]",
    "<a b=\"c\" />",
    "<a b='c'>d</a>",
    "<a:b c:d=\"e\" />",
    "<a.b.c />",
    "<></>",
    "<>a</>",
    "<a>&amp;&lt;&#123;&#x7B;</a>",
    "<a b={c} {...d} />",
    "<a b=<c /> />",
    "<a /* c */ b />",
    "<a>{/* c */}</a>",
    "<a>{}</a>",
    "<a b=\"\\\" />",
    "<a>\\</a>",
    "<a>'</a>",
    "<a>\"</a>",
    "<a b=\"\n\" />",
    "x as y",
    "x!",
    "<T>(x)",
    "let x: number",
    "function f<T>(){}",
    "enum E {}",
    "interface I {}",
    "type T = 1",
    "declare var x",
    "abstract class C {}",
    "x satisfies y",
    "f<T>(x)",
    "a!.b",
    "public x",
    "namespace N {}",
    "class A implements B {}",
    "function f(this: T){}",
    "let x = y as const",
    "(x: number) => x",
];

const SEPARATORS: &[&str] = &[
    "", " ", "\n", ";", "\n;", "\r\n", "\u{2028}", "/*\n*/", "// c\n", "\t", ";\n", "\r",
    " /*x*/ ", "\u{2029}",
];

const WRAPPERS: &[&str] = &[
    "@",
    "class A { static #p(){ @ } }",
    "class A { get x(){ @ } }",
    "class A { set x(v){ @ } }",
    "class A { static async *m(){ @ } }",
    "class A { x = (() => { @ })() }",
    "class A extends B { constructor(){ super(); @ } }",
    "({ get a(){ @ }, set a(v){} })",
    "({ async *m(){ @ } })",
    "l1: l2: for(;;) { @ }",
    "x = async (a) => { @ }",
    "x = async a => { @ }",
    "a?.b(() => { @ })",
    "import('x').then(async () => { @ })",
    "function f(){ new.target; @ }",
    "<a.b c:d=\"e\" {...f}>{(() => { @ })()}</a.b>",
    "<>{(function*(){ @ })()}</>",
    "tag`${(function*(){ @ })()}`",
    "#!/usr/bin/env node\n@",
    "\u{feff}@",
    "try { @ } catch { } finally { }",
    "try {} catch ({a}) { @ }",
    "switch (a) { default: { @ } }",
    "if (a) b; else { @ }",
    "function* g(){ yield* (function*(){ @ })() }",
    "async function* f(){ for await (const x of y) { @ } }",
    "\"use strict\"; function* g(){ @ }",
    "export async function f(){ @ }",
    "x = { [(() => { @ })()]: 1 }",
    "class A { [(() => { @ })()](){} }",
    "class A { static { await: { @ } } }",
    "async function f(){ class B { [await x](){ @ } } }",
    "function* g(){ class B { [yield](){ @ } } }",
    "async () => { using a = b; @ }",
    "label: { @ }",
    "x = { async *[a](){ @ } }",
    "async function f(){ for (await using x of y) { @ } }",
    "<a b={(() => { @ })()} />",
];

const HIDERS: &[&str] = &[
    "/`/;\n@\n;/`/",
    "/'/,@,'/'",
    "/\"/,@,\"/\"",
    "/[/]`/;\n@\n;/`/",
    "/\\/`/;\n@\n;/`/",
    "/=`/;\n@\n;/`/",
    "<a>`</a>;\n@\n;`",
    "<a b=\"`\">`</a>;\n@\n;`",
    "{}\n/`/;\n@\n;/`/",
    "/@/",
    "<a>\n@\n</a>",
    "< a>@</ a>",
    "/[@]/",
    "`/;\n@\n;/`",
    "<!--`\n@\n`",
    "-->`\n@\n`",
    "\n/`/;\n@\n;/`/",
    "\n<a>`</a>;\n@\n;`",
    "(/`/);\n@\n;/`/",
    "/`/g;\n@\n;/`/",
    "/'/\n@\n'/'",
    "\u{2028}/`/;\n@\n;/`/",
    "/*\n*//`/;\n@\n;/`/",
];

const TOKENS: &[&str] = &[
    "/",
    "`",
    "'",
    "\"",
    "<",
    ">",
    "{",
    "}",
    "(",
    ")",
    "[",
    "]",
    "\n",
    "\u{2028}",
    "<!--",
    "-->",
    "#!",
    "@",
    "\\u0061wait",
    "yield",
    "await",
    "of",
    "let",
    "async",
    "=>",
    "?.",
    "?",
    ":",
    ";",
    "break",
    "import",
    "export",
    "from",
    "default",
    "class",
    "function",
    "*",
    "#x",
    "new",
    "</",
    "/>",
    "${",
    "\\",
    "\r",
    "/*",
    "*/",
    "//",
    "get",
    "static",
    "using",
    ".",
    ",",
    "=",
];

fn region(rng: &mut Rng, depth: usize) -> String {
    match rng.below(8) {
        0 => format!("{}x{}", "(".repeat(depth), ")".repeat(depth)),
        1 => format!("{}x{}", "[".repeat(depth), "]".repeat(depth)),
        2 => format!("{}1{}", "({a:".repeat(depth), "})".repeat(depth)),
        3 => "(".repeat(depth),
        4 => format!("{}x", "!".repeat(depth)),
        5 => format!("{}x", "a?b:".repeat(depth)),
        6 => format!("{}x", "()=>".repeat(depth)),
        _ => format!("{}x{}", "`${".repeat(depth), "}`".repeat(depth)),
    }
}

pub(crate) fn depth(rng: &mut Rng, depths: &[usize]) -> usize {
    depths.get(rng.below(depths.len())).copied().unwrap_or(300)
}

fn sites(rng: &mut Rng, pool: &[&str], most: usize) -> String {
    let mut text = String::new();
    for _ in 0..1 + rng.below(most) {
        text.push_str(rng.pick(pool));
        text.push_str(rng.pick(SEPARATORS));
    }
    text
}

fn hidden(rng: &mut Rng, depths: &[usize]) -> String {
    let n = depth(rng, depths);
    let deep = region(rng, n);
    rng.pick(HIDERS).replace('@', &deep)
}

fn multi(rng: &mut Rng, depths: &[usize]) -> String {
    let before = sites(rng, DOUBTFUL, 4);
    let middle = if rng.chance(30) {
        sites(rng, GRAMMAR, 1)
    } else {
        String::new()
    };
    let after = if rng.chance(30) {
        sites(rng, DOUBTFUL, 2)
    } else {
        String::new()
    };
    let body = format!(
        "{before}{middle}{}{}{after}",
        hidden(rng, depths),
        rng.pick(SEPARATORS)
    );
    rng.pick(WRAPPERS).replacen('@', &body, 1)
}

fn grammar(rng: &mut Rng, depths: &[usize]) -> String {
    let before = sites(rng, GRAMMAR, 3);
    let mixed = if rng.chance(40) {
        sites(rng, DOUBTFUL, 1)
    } else {
        String::new()
    };
    let body = format!("{before}{mixed}{}", hidden(rng, depths));
    rng.pick(WRAPPERS).replacen('@', &body, 1)
}

fn boundary(text: &str, mut at: usize) -> usize {
    while at < text.len() && !text.is_char_boundary(at) {
        at += 1;
    }
    at.min(text.len())
}

fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut previous: Option<bool> = None;
    for (index, ch) in text.char_indices() {
        let word = ch.is_alphanumeric() || ch == '_' || ch == '$';
        if previous.is_some() && (previous != Some(word) || !word) {
            out.push(&text[start..index]);
            start = index;
        }
        previous = Some(word);
    }
    out.push(&text[start..]);
    out
}

fn mutate_tokens(rng: &mut Rng, parts: &mut Vec<String>) {
    for _ in 0..1 + rng.below(5) {
        let at = rng.below(parts.len().max(1));
        match rng.below(5) {
            0 => parts.insert(at.min(parts.len()), rng.pick(TOKENS).to_owned()),
            1 if !parts.is_empty() => {
                parts.remove(at);
            }
            2 if parts.len() > 1 => {
                let other = rng.below(parts.len());
                parts.swap(at, other);
            }
            3 => parts.insert(at.min(parts.len()), rng.pick(DOUBTFUL).to_owned()),
            _ => parts.insert(at.min(parts.len()), rng.pick(GRAMMAR).to_owned()),
        }
    }
}

fn mutant(rng: &mut Rng, depths: &[usize], corpus: &[String]) -> String {
    let Some(file) = corpus.get(rng.below(corpus.len())) else {
        return multi(rng, depths);
    };
    let start = boundary(file, rng.below(file.len().max(1)));
    let end = boundary(file, start + 100 + rng.below(1400));
    let slice = &file[start..end];
    let mut parts: Vec<String> = tokens(slice).into_iter().map(str::to_owned).collect();
    mutate_tokens(rng, &mut parts);
    let n = depth(rng, depths);
    let deep = if rng.chance(50) {
        hidden(rng, depths)
    } else {
        region(rng, n)
    };
    let at = rng.below(parts.len() + 1);
    parts.insert(at.min(parts.len()), deep);
    parts.concat()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mix {
    Multi,
    Grammar,
    Mutate,
}

impl Mix {
    pub fn from_name(name: &str) -> Option<Mix> {
        Some(match name {
            "multi" => Mix::Multi,
            "grammar" => Mix::Grammar,
            "mutate" => Mix::Mutate,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Mix::Multi => "multi",
            Mix::Grammar => "grammar",
            Mix::Mutate => "mutate",
        }
    }
}

pub const FULL_DEPTHS: [usize; 4] = [300, 500, 800, 1_500];
pub const SAMPLE_DEPTHS: [usize; 2] = [300, 400];
const CORPUS_FILES: usize = 13;

fn corpus() -> &'static [String] {
    static CORPUS: OnceLock<Vec<String>> = OnceLock::new();
    CORPUS.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let game = root.join("fixtures/real-world/game.js");
        let mut files: Vec<PathBuf> = Some(game)
            .filter(|path| path.is_file())
            .into_iter()
            .collect();
        let expected = CORPUS_FILES - 1 + files.len();
        let mut npm: Vec<PathBuf> = std::fs::read_dir(root.join("fixtures/corpus-npm"))
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        npm.sort();
        files.extend(npm);
        let texts: Vec<String> = files
            .iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect();
        assert_eq!(texts.len(), expected, "the mutation corpus is incomplete");
        texts
    })
}

fn generate(mix: Mix, rng: &mut Rng, depths: &[usize]) -> String {
    match mix {
        Mix::Multi => multi(rng, depths),
        Mix::Grammar => grammar(rng, depths),
        Mix::Mutate => mutant(rng, depths, corpus()),
    }
}

pub(crate) fn seeded(seed: u64, index: u64) -> Rng {
    Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(index))
}

pub fn program(mix: Mix, seed: u64, index: u64) -> (String, String) {
    let mut rng = seeded(seed, index);
    (generate(mix, &mut rng, &FULL_DEPTHS), mix.name().to_owned())
}

pub fn sampled(seed: u64, index: u64) -> (String, String) {
    let mut rng = seeded(seed, index);
    let mix = match rng.below(10) {
        0..=3 => Mix::Multi,
        4..=6 => Mix::Grammar,
        _ => Mix::Mutate,
    };
    let text = generate(mix, &mut rng, &SAMPLE_DEPTHS);
    (text, format!("mix {}", mix.name()))
}
