const SETTER_PREFIX: &str = "set_";
const MIN_STEM: usize = 3;
const SHORT_STEM: usize = 3;
const GENERATED_PREFIXES: &[&str] = &["hasrequired", "requiremodule"];
const GENERIC_STEMS: &[&str] = &[
    "arr",
    "bool",
    "callback",
    "cb",
    "cls",
    "component",
    "content",
    "ctor",
    "data",
    "default",
    "el",
    "err",
    "exports",
    "fn",
    "func",
    "get",
    "hasrequired",
    "instance",
    "item",
    "module",
    "num",
    "obj",
    "predicate",
    "re",
    "req",
    "require",
    "requiremodule",
    "res",
    "result",
    "set",
    "str",
    "tmp",
    "v",
    "value",
];

fn is_plain(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
        && !name.starts_with(|ch: char| ch.is_ascii_digit())
}

fn base(name: &str) -> &str {
    name.trim_start_matches(['_', '$'])
        .trim_end_matches(|ch: char| ch.is_ascii_digit() || ch == '_')
}

fn looks_minified(base: &str) -> bool {
    if base.len() > SHORT_STEM {
        return false;
    }
    let mixed = base.chars().any(|ch| ch.is_ascii_uppercase())
        && base.chars().skip(1).any(|ch| ch.is_ascii_uppercase());
    let vowel = base
        .chars()
        .any(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y'));
    let digit = base.chars().any(|ch| ch.is_ascii_digit());
    mixed || digit || !vowel || base.contains('$') || base.contains('_')
}

pub fn meaningful(name: &str) -> bool {
    if !is_plain(name)
        || name.starts_with("_0x")
        || name.starts_with(SETTER_PREFIX)
        || name.contains("__")
    {
        return false;
    }
    let base = base(name);
    if base.len() < MIN_STEM || looks_minified(base) {
        return false;
    }
    let folded = base.to_ascii_lowercase();
    let wrapper = GENERATED_PREFIXES
        .iter()
        .any(|prefix| folded.starts_with(prefix));
    !wrapper && !GENERIC_STEMS.contains(&folded.as_str())
}

pub fn is_numbered(stem: &str, prefix: &str) -> bool {
    stem.strip_prefix(prefix)
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}
