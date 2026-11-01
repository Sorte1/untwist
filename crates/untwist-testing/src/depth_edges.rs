use crate::depth_grid::Rng;

const NAME_PARTS: &[&str] = &[
    "a",
    "b",
    "z",
    "Q",
    "$",
    "_",
    "0",
    "9",
    "é",
    "ß",
    "中",
    "𝑥",
    "\\u0061",
    "\\u{62}",
    "\\u{0000063}",
    "\\u00e9",
    "\\u{1d465}",
    "\\x",
    "\\u12",
    "\\",
];

const WORDS: &[&str] = &[
    "as",
    "satisfies",
    "implements",
    "interface",
    "await",
    "yield",
    "let",
    "of",
    "async",
    "class",
    "function",
    "static",
    "get",
    "set",
    "using",
    "import",
    "export",
    "default",
    "return",
    "typeof",
    "in",
    "new",
    "this",
    "a\\u0073",
    "\\u0061s",
    "l\\u0065t",
    "aw\\u0061it",
    "yi\\u{65}ld",
    "st\\u0061tic",
    "\\u0061sync",
    "g\\u0065t",
    "readonl\\u0079",
    "\\u0069mplements",
    "s\\u0061tisfies",
    "interf\\u0061ce",
    "class C { static [k: T]; }",
    "class C { st\\u0061tic [k: T]; }",
    "class C { readonly [k: string]: T }",
];

const BLANKS: &[&str] = &[
    " ", " ", " ", "\t", "\n", "\r\n", "\r", "\u{b}", "\u{c}", "\u{a0}", "\u{feff}", "\u{2028}",
    "\u{2029}", "\u{3000}", "\u{1680}", "\u{2007}",
];

const STRING_PARTS: &[&str] = &[
    "x",
    "ab",
    " ",
    "é",
    "中",
    "😀",
    "\\'",
    "\\\"",
    "\\\\",
    "\\\n",
    "\\\r\n",
    "\\\r",
    "\\u{1F600}",
    "\\x41",
    "\\u2028",
    "\u{2028}",
    "\u{2029}",
    "\\é",
    "`",
    "${",
    "/*",
    "//",
    "<!--",
];

const TEMPLATE_PARTS: &[&str] = &[
    "x",
    "ab",
    " ",
    "\n",
    "\r\n",
    "é",
    "$",
    "${a}",
    "${`in${b}`}",
    "${{a:1}}",
    "\\`",
    "\\$",
    "\\${",
    "\\\\",
    "\\u{61}",
    "'",
    "\"",
    "/",
    "}",
    "{",
];

const COMMENT_PARTS: &[&str] = &[
    "x", " ", "*", "**", "/", "*/x", "é", "\u{2028}", "\n", "\r\n", "\r", "<!--", "-->", "\\",
];

const PIECES: &[&str] = &[
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ";",
    ",",
    "=>",
    "?",
    ":",
    "=",
    "+",
    "-",
    ".",
    "...",
    "?.",
    "++",
    "--",
    "<",
    ">",
    "</",
    "/>",
    "@",
    "#x",
    "*",
    "!",
    "&&",
    "||",
    "??",
    "**=",
    ">>>=",
    "a / b",
    "a / b / c",
    "x = /ab+c/gi",
    "(a) / 2 / 3",
    "return /x/",
    "}\n/x/",
    "x++ / 2",
    "[1] / 2",
    "/[/]/.test(s)",
    "a\n/b/g",
    "a /= 2",
    "/=/",
    "if (a) /x/",
    "yield /x/",
    "await /x/",
    "of /x/",
    "let /x/",
    "x = a\n/b/",
    "1 / 2",
    "0x1f / 2",
    "1e5 / 2",
    ".5 / 2",
    "1.e-3",
    "if (",
    "else ",
    "function f(",
    "class C {",
    "class C extends D<T> {",
    "async ",
    "await ",
    "yield ",
    "let ",
    "x satisfies y",
    "x as y",
    "import x = require('y')",
    "export default interface I {}",
    "<div a=\"b\">{x}</div>",
    "<a></a>",
    "<>t</>",
    "`t${x}t`",
    "a?b:c",
    "f(...a)",
    "(a, b) => a",
    "async (a) => a",
    "x[0]",
    "{ [k: string]: T }",
    "class C { [k: string]: T }",
    "for (let of of x)",
    "label: for(;;) break label",
    "(a;b",
    "(x\n)y",
    "if (a) (b",
    "if (a) ([b, c]) => d",
    "x => (a",
    "x => ({a}) => b",
    "((a) => b",
    "(;a",
    "(\nbreak",
    "(import",
    "while (x) (async",
    "for (;;) (let",
    "do (a",
    "else ([a",
    "(a\n++b",
    "(await function() {} / x",
    "(await class {} / x / y",
    "(await /x/",
    "(await\n/x/g",
    "(yield function* () {} / x",
    "(yield /x/",
    "(await (",
    "(async function() { (await function() {} / a) }",
];

pub fn program(seed: u64, index: u64) -> String {
    let mut rng = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(index));
    let mut out = String::new();
    if rng.below(20) == 0 {
        out.push_str("#!");
        push_comment_body(&mut rng, &mut out);
        out.push('\n');
    }
    for _ in 0..rng.below(8) {
        out.push(if rng.below(2) == 0 { ' ' } else { 'w' });
    }
    for _ in 0..1 + rng.below(40) {
        piece(&mut rng, &mut out);
        if rng.below(3) != 0 {
            blanks(&mut rng, &mut out, 4);
        }
    }
    if rng.below(3) == 0 {
        truncate(&mut rng, &mut out);
    }
    out
}

fn piece(rng: &mut Rng, out: &mut String) {
    match rng.below(10) {
        0 | 1 => name(rng, out),
        2 => out.push_str(WORDS[rng.below(WORDS.len())]),
        3 => blanks(rng, out, 40),
        4 => string(rng, out),
        5 => template(rng, out),
        6 => comment(rng, out),
        7 => number(rng, out),
        _ => out.push_str(PIECES[rng.below(PIECES.len())]),
    }
}

fn name(rng: &mut Rng, out: &mut String) {
    let most = if rng.below(4) == 0 { 100 } else { 12 };
    let length = 1 + rng.below(most);
    for _ in 0..length {
        let part = if rng.below(6) == 0 {
            NAME_PARTS[rng.below(NAME_PARTS.len())]
        } else {
            NAME_PARTS[rng.below(6)]
        };
        out.push_str(part);
    }
}

fn blanks(rng: &mut Rng, out: &mut String, most: usize) {
    for _ in 0..1 + rng.below(most) {
        let blank = if rng.below(3) == 0 {
            BLANKS[rng.below(BLANKS.len())]
        } else {
            " "
        };
        out.push_str(blank);
    }
}

fn parts(rng: &mut Rng, out: &mut String, table: &[&str], most: usize) {
    for _ in 0..rng.below(most) {
        let part = if rng.below(3) == 0 {
            table[rng.below(table.len())]
        } else {
            table[0]
        };
        out.push_str(part);
    }
}

fn string(rng: &mut Rng, out: &mut String) {
    let quote = if rng.below(2) == 0 { '\'' } else { '"' };
    out.push(quote);
    parts(rng, out, STRING_PARTS, 60);
    match rng.below(8) {
        0 => out.push('\n'),
        1 => {}
        _ => out.push(quote),
    }
}

fn template(rng: &mut Rng, out: &mut String) {
    out.push('`');
    parts(rng, out, TEMPLATE_PARTS, 60);
    if rng.below(8) != 0 {
        out.push('`');
    }
}

fn push_comment_body(rng: &mut Rng, out: &mut String) {
    for _ in 0..rng.below(50) {
        let part = if rng.below(3) == 0 {
            COMMENT_PARTS[1 + rng.below(COMMENT_PARTS.len() - 1)]
        } else {
            COMMENT_PARTS[0]
        };
        if part.contains(['\n', '\r', '\u{2028}']) || part.starts_with("*/") {
            continue;
        }
        out.push_str(part);
    }
}

fn comment(rng: &mut Rng, out: &mut String) {
    match rng.below(5) {
        0 | 1 => {
            out.push_str("//");
            push_comment_body(rng, out);
            out.push_str(["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"][rng.below(5)]);
        }
        2 => {
            out.push_str("/*");
            parts(rng, out, COMMENT_PARTS, 60);
            if rng.below(8) != 0 {
                out.push_str("*/");
            }
        }
        3 => {
            out.push_str(["<!--", "x <!--", "\n<!--"][rng.below(3)]);
            push_comment_body(rng, out);
            out.push('\n');
        }
        _ => {
            out.push_str(["\n-->", "-->", "\n  -->", "/*\n*/-->", "a-->b"][rng.below(5)]);
            push_comment_body(rng, out);
            out.push('\n');
        }
    }
}

fn number(rng: &mut Rng, out: &mut String) {
    const NUMBERS: &[&str] = &[
        "0",
        "1",
        "123456789012",
        "0x1F",
        "0b101",
        "0o17",
        "1e10",
        "1E+5",
        "2e-3",
        "1.5",
        ".5",
        "5.",
        "1_000",
        "10n",
        "0.0e0",
        "08",
        "1..toString",
    ];
    out.push_str(NUMBERS[rng.below(NUMBERS.len())]);
}

fn truncate(rng: &mut Rng, out: &mut String) {
    if out.is_empty() {
        return;
    }
    let mut cut = rng.below(out.len());
    while !out.is_char_boundary(cut) {
        cut -= 1;
    }
    out.truncate(cut);
}
