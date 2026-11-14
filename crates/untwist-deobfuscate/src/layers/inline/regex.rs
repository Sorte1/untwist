use untwist_eval::builtins::regexp_syntax;

const PORTABLE_FLAGS: &str = "gimsuy";

const PORTABLE_GROUP_OPENERS: [char; 4] = [':', '=', '!', '<'];

pub(super) fn holds_everywhere(pattern: &str, flags: &str) -> bool {
    portable_flags(flags)
        && portable_syntax(pattern)
        && regexp_syntax::admitted(pattern, grammar_flags(flags))
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

fn portable_syntax(pattern: &str) -> bool {
    let chars: Vec<char> = pattern.chars().collect();
    let mut names: Vec<String> = Vec::new();
    let mut in_class = false;
    let mut at = 0;
    while at < chars.len() {
        match chars[at] {
            '\\' => {
                if matches!(chars.get(at + 1), Some('p' | 'P' | 'k')) {
                    return false;
                }
                at += 1;
            }
            '[' => in_class = true,
            ']' => in_class = false,
            '(' if !in_class && chars.get(at + 1) == Some(&'?') => {
                match group_name(&chars, at + 2) {
                    Group::Plain => {}
                    Group::Other => return false,
                    Group::Named(name) if names.contains(&name) => return false,
                    Group::Named(name) => names.push(name),
                }
            }
            _ => {}
        }
        at += 1;
    }
    true
}

enum Group {
    Plain,
    Named(String),
    Other,
}

fn group_name(chars: &[char], at: usize) -> Group {
    let Some(&opener) = chars.get(at) else {
        return Group::Other;
    };
    if !PORTABLE_GROUP_OPENERS.contains(&opener) {
        return Group::Other;
    }
    if opener != '<' {
        return Group::Plain;
    }
    let rest = &chars[at + 1..];
    if matches!(rest.first(), Some('=' | '!')) {
        return Group::Other;
    }
    let name: String = rest.iter().take_while(|ch| **ch != '>').collect();
    Group::Named(name)
}
