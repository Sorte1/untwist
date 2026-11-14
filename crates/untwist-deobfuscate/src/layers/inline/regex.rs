use untwist_eval::builtins::regexp_syntax;

const PORTABLE_FLAGS: &str = "gimsuy";

pub(super) fn holds_everywhere(pattern: &str, flags: &str) -> bool {
    portable_flags(flags) && regexp_syntax::admitted(pattern, grammar_flags(flags))
}

fn grammar_flags(flags: &str) -> &'static str {
    match flags.contains('u') {
        true => "u",
        false => "",
    }
}

fn portable_flags(flags: &str) -> bool {
    let mut seen = String::new();
    for flag in flags.chars() {
        if !PORTABLE_FLAGS.contains(flag) || seen.contains(flag) {
            return false;
        }
        seen.push(flag);
    }
    true
}
