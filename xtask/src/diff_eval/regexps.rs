use super::pool::Rng;

const MAX_DEPTH: usize = 3;
const MAX_TERMS: usize = 4;
const NEAR_MISS_PERCENT: u64 = 15;
const BAD_FLAGS_PERCENT: u64 = 10;
const SUBJECT_MAX_LEN: usize = 10;

const LITERALS: &[&str] = &[
    "a", "b", "c", "A", "B", "0", "1", "-", " ", "_", ",", "k", "\u{e9}", "\u{df}", "\u{3a3}",
];

const ESCAPES: &[&str] = &[
    "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\n", "\\t", "\\r", "\\f", "\\v", "\\x41", "\\x61",
    "\\u00e9", "\\u0041", "\\.", "\\*", "\\/", "\\\\", "\\(", "\\[", "\\{", "\\|", "\\^", "\\$",
    "\\+", "\\?",
];

const CLASSES: &[&str] = &[
    "[abc]",
    "[a-c]",
    "[^ab]",
    "[\\d_]",
    "[-a]",
    "[a-]",
    "[0-9]",
    "[\\w-]",
    "[ \\n]",
    "[A-Z]",
    "[\\u00e0-\\u00ff]",
    "[.*]",
    "[\\]]",
    "[^\\s]",
    "[\\x41-\\x43]",
];

const ASSERTIONS: &[&str] = &["^", "$", "\\b", "\\B"];

const QUANTIFIERS: &[&str] = &[
    "*", "+", "?", "{2}", "{0,}", "{1,3}", "{0,1}", "{3,}", "{1000}", "{0,1000}",
];

const NEAR_MISSES: &[&str] = &[
    "(?=a)",
    "(?!a)",
    "(?<=a)",
    "(?<!a)",
    "(?<n>a)\\k<n>",
    "\\p{L}",
    "\\P{Lu}",
    "\\cA",
    "\\0",
    "\\k",
    "\\e",
    "\\-",
    "[]",
    "[^]",
    "a{2,1}",
    "^*",
    "a**",
    "(",
    ")",
    "[",
    "{",
    "}",
    "]",
    "a{1001}",
    "[b-a]",
    "\u{1f600}",
    "(?i:a)",
    "\\u{61}",
    "[\\d-z]",
    "\\1",
    "a{,2}",
    "x{",
    "\\8",
    "[[a]]",
    "(?:",
    "|*",
    "\\u12",
    "\\xg",
];

const FLAG_LETTERS: &[char] = &['g', 'i', 'm', 's', 'u', 'v'];

const BAD_FLAGS: &[&str] = &["uv", "vu", "guv", "gg", "y", "d", "x", "iu", "sv", "gimsuy"];

const SUBJECT_UNITS: &[&str] = &[
    "a", "b", "c", "A", "B", "C", "0", "1", "9", " ", "-", "_", ",", ".", "k", "K", "\n",
    "\u{2028}", "\u{e9}", "\u{c9}", "\u{df}", "\u{3c3}", "\u{3a3}", "\u{212a}", "$", "*",
];

fn one<'s>(rng: &mut Rng, list: &[&'s str]) -> &'s str {
    list[rng.below(list.len())]
}

struct Atom {
    quantifiable: bool,
    quantified_inside: bool,
}

fn group(rng: &mut Rng, depth: usize, open: &str, out: &mut String) -> Atom {
    out.push_str(open);
    let quantified_inside = disjunction(rng, depth + 1, out);
    out.push(')');
    Atom {
        quantifiable: true,
        quantified_inside,
    }
}

fn atom(rng: &mut Rng, depth: usize, out: &mut String) -> Atom {
    let plain = Atom {
        quantifiable: true,
        quantified_inside: false,
    };
    match rng.below(if depth < MAX_DEPTH { 8 } else { 5 }) {
        0 | 1 => out.push_str(one(rng, LITERALS)),
        2 => out.push_str(one(rng, ESCAPES)),
        3 => out.push_str(one(rng, CLASSES)),
        4 => out.push('.'),
        5 => return group(rng, depth, "(", out),
        6 => return group(rng, depth, "(?:", out),
        _ => {
            out.push_str(one(rng, ASSERTIONS));
            return Atom {
                quantifiable: false,
                quantified_inside: false,
            };
        }
    }
    plain
}

fn term(rng: &mut Rng, depth: usize, out: &mut String) -> bool {
    let atom = atom(rng, depth, out);
    let nests = atom.quantified_inside;
    if atom.quantifiable && !nests && rng.chance(35) {
        out.push_str(one(rng, QUANTIFIERS));
        if rng.chance(25) {
            out.push('?');
        }
        return true;
    }
    nests
}

fn alternative(rng: &mut Rng, depth: usize, out: &mut String) -> bool {
    let mut quantified = false;
    for _ in 0..rng.below(MAX_TERMS) + 1 {
        quantified |= term(rng, depth, out);
    }
    quantified
}

fn disjunction(rng: &mut Rng, depth: usize, out: &mut String) -> bool {
    let mut quantified = alternative(rng, depth, out);
    while rng.chance(20) {
        out.push('|');
        quantified |= alternative(rng, depth, out);
    }
    quantified
}

fn with_backreference(rng: &mut Rng, pattern: String) -> String {
    let groups = pattern.matches('(').count() - pattern.matches("(?").count();
    if groups == 0 || !rng.chance(20) {
        return pattern;
    }
    format!("{pattern}\\{}", rng.below(groups) + 1)
}

fn pattern(rng: &mut Rng) -> String {
    let mut out = String::new();
    disjunction(rng, 0, &mut out);
    let out = with_backreference(rng, out);
    if !rng.chance(NEAR_MISS_PERCENT) {
        return out;
    }
    let at = rng.below(out.chars().count() + 1);
    let mut chars: Vec<char> = out.chars().collect();
    let miss: Vec<char> = one(rng, NEAR_MISSES).chars().collect();
    chars.splice(at..at, miss);
    chars.into_iter().collect()
}

fn flags(rng: &mut Rng) -> String {
    if rng.chance(BAD_FLAGS_PERCENT) {
        return one(rng, BAD_FLAGS).to_string();
    }
    let mut letters: Vec<char> = FLAG_LETTERS
        .iter()
        .copied()
        .filter(|_| rng.chance(30))
        .collect();
    if letters.contains(&'u') && letters.contains(&'v') {
        letters.retain(|letter| *letter != if rng.chance(50) { 'u' } else { 'v' });
    }
    for index in (1..letters.len()).rev() {
        letters.swap(index, rng.below(index + 1));
    }
    letters.into_iter().collect()
}

fn subject(rng: &mut Rng) -> String {
    (0..rng.below(SUBJECT_MAX_LEN + 1))
        .map(|_| one(rng, SUBJECT_UNITS))
        .collect()
}

fn operation(rng: &mut Rng) -> (&'static str, &'static str) {
    const OPERATIONS: &[(&str, &str)] = &[
        ("test", "re.test(s)"),
        ("exec", "re.exec(s)"),
        ("replace", "s.replace(re, '[$&|$1|$`]')"),
        ("replace fn", "s.replace(re, function (m, p, o) { return '<' + m + ':' + o + '>'; })"),
        ("split", "s.split(re)"),
        ("match", "s.match(re)"),
        ("search", "s.search(re)"),
        ("toString", "String(re)"),
        ("source", "re.source"),
        ("flags", "re.flags"),
        ("lastIndex", "[re.test(s), re.lastIndex, re.test(s), re.lastIndex]"),
        ("exec loop", "(function () { var out = [], m, n = 0; while (n++ < 5 && (m = re.exec(s))) { out.push(m.index, m[0], m.length); if (!re.global) break; } return out; })()"),
    ];
    OPERATIONS[rng.below(OPERATIONS.len())]
}

fn literal(text: &str) -> String {
    serde_json::Value::String(text.to_string()).to_string()
}

pub fn regexp_case(rng: &mut Rng) -> (String, String) {
    let source = pattern(rng);
    let flag_text = flags(rng);
    let text = subject(rng);
    let (name, call) = operation(rng);
    let construct = if rng.chance(50) {
        "new RegExp"
    } else {
        "RegExp"
    };
    let decode = format!(
        "(function () {{ var re = {construct}({}, {}); var s = {}; var r = {call}; return [r]; }})()",
        literal(&source),
        literal(&flag_text),
        literal(&text)
    );
    (format!("regexp {name}"), decode)
}
