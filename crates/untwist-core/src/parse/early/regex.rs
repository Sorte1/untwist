use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_regular_expression::ast::{
    BoundaryAssertion, BoundaryAssertionKind, CapturingGroup, IgnoreGroup, UnicodePropertyEscape,
};
use oxc_regular_expression::visit::{walk, Visit};
use oxc_regular_expression::{LiteralParser, Options};

const V8_INFINITY: u64 = 2_147_483_647;
const BRACED_ESCAPES: &[char] = &['u', 'p', 'P', 'q'];
const SCRIPT_KEYS: [&str; 4] = ["sc", "Script", "scx", "Script_Extensions"];
const SCRIPTS_NODE_LACKS: [&str; 2] = ["Hrkt", "Katakana_Or_Hiragana"];
const ALIASES_NODE_READS: [(&str, &str); 1] = [("WSpace", "White_Space")];

#[derive(Default)]
struct Groups {
    names: Vec<String>,
    modifiers: bool,
    buffer_assertion: bool,
    unknown_script: bool,
}

impl<'a> Visit<'a> for Groups {
    fn visit_capturing_group(&mut self, it: &CapturingGroup<'a>) {
        if let Some(name) = &it.name {
            self.names.push(name.to_string());
        }
        walk::walk_capturing_group(self, it);
    }

    fn visit_ignore_group(&mut self, it: &IgnoreGroup<'a>) {
        self.modifiers |= it.modifiers.is_some();
        walk::walk_ignore_group(self, it);
    }

    fn visit_boundary_assertion(&mut self, it: &BoundaryAssertion) {
        self.buffer_assertion |= matches!(
            it.kind,
            BoundaryAssertionKind::StartBuffer
                | BoundaryAssertionKind::EndBuffer
                | BoundaryAssertionKind::EndBufferOptionalNewline
        );
    }

    fn visit_unicode_property_escape(&mut self, it: &UnicodePropertyEscape<'a>) {
        let script = SCRIPT_KEYS.contains(&it.name.as_str());
        self.unknown_script |= script
            && it
                .value
                .as_ref()
                .is_some_and(|value| SCRIPTS_NODE_LACKS.contains(&value.as_str()));
    }
}

fn clamped(digits: &str) -> &str {
    let significant = digits.trim_start_matches('0');
    let large = significant.len() > 10
        || significant
            .parse::<u64>()
            .is_ok_and(|value| value > V8_INFINITY);
    match large {
        true => "2147483647",
        false => digits,
    }
}

fn braced_quantifier(rest: &str) -> Option<(usize, String)> {
    let body = rest.strip_prefix('{')?;
    let close = body.find('}')?;
    let inner = &body[..close];
    let (min, max) = match inner.split_once(',') {
        Some((min, max)) => (min, Some(max)),
        None => (inner, None),
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(min) || max.is_some_and(|max| !max.is_empty() && !digits(max)) {
        return None;
    }
    let rewritten = match max {
        Some("") => format!("{{{},}}", clamped(min)),
        Some(max) => format!("{{{},{}}}", clamped(min), clamped(max)),
        None => format!("{{{}}}", clamped(min)),
    };
    Some((close + 2, rewritten))
}

fn property_alias(rest: &str) -> Option<(usize, String)> {
    let body = rest.strip_prefix('{')?;
    let close = body.find('}')?;
    let name = &body[..close];
    ALIASES_NODE_READS
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, canonical)| (close + 2, format!("{{{canonical}}}")))
}

fn as_node_reads(pattern: &str, unicode: bool) -> Cow<'_, str> {
    if !pattern.contains('{') {
        return Cow::Borrowed(pattern);
    }
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(next) = rest.chars().next() {
        if next == '\\' {
            let escaped = rest[1..].chars().next();
            let width = 1 + escaped.map_or(0, char::len_utf8);
            out.push_str(&rest[..width]);
            rest = &rest[width..];
            let property = unicode && matches!(escaped, Some('p' | 'P'));
            let replaced = property.then(|| property_alias(rest)).flatten();
            if let Some((used, text)) = replaced {
                out.push_str(&text);
                rest = &rest[used..];
                continue;
            }
            if unicode && escaped.is_some_and(|escaped| BRACED_ESCAPES.contains(&escaped)) {
                if let Some(at) = rest.find('}').filter(|_| rest.starts_with('{')) {
                    out.push_str(&rest[..=at]);
                    rest = &rest[at + 1..];
                }
            }
            continue;
        }
        if let Some((used, text)) = braced_quantifier(rest) {
            out.push_str(&text);
            rest = &rest[used..];
            continue;
        }
        out.push(next);
        rest = &rest[next.len_utf8()..];
    }
    Cow::Owned(out)
}

pub fn problem(pattern: &str, flags: &str) -> Option<String> {
    let unicode = flags.contains('u') || flags.contains('v');
    let read = as_node_reads(pattern, unicode);
    let alloc = Allocator::default();
    let parsed = LiteralParser::new(&alloc, &read, Some(flags), Options::default()).parse();
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(diagnostic) => return Some(diagnostic.message.to_string()),
    };
    let mut groups = Groups::default();
    groups.visit_pattern(&parsed);
    if groups.modifiers {
        return Some("node 22 does not read regular expression modifiers".to_string());
    }
    if groups.buffer_assertion {
        return Some(
            "node 22 does not read `\\A`, `\\z` or `\\Z` in a regular expression".to_string(),
        );
    }
    if groups.unknown_script {
        return Some("node 22 does not read the script value `Hrkt`".to_string());
    }
    let mut names = groups.names;
    names.sort_unstable();
    names.windows(2).any(|pair| pair[0] == pair[1]).then(|| {
        "node 22 does not read a capture group name used twice in a regular expression".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braced_counts_above_v8s_limit_are_clamped_and_escapes_are_kept() {
        assert_eq!(
            as_node_reads("a{99999999999999999999}", false),
            "a{2147483647}"
        );
        assert_eq!(
            as_node_reads("a{3000000000,2147483648}", false),
            "a{2147483647,2147483647}"
        );
        assert_eq!(as_node_reads("a{1,}", false), "a{1,}");
        assert_eq!(
            as_node_reads("a{000000000001,2}", false),
            "a{000000000001,2}"
        );
        assert_eq!(as_node_reads("\\u{99999999999}", true), "\\u{99999999999}");
        assert_eq!(as_node_reads("\\u{99999999999}", false), "\\u{2147483647}");
        assert_eq!(as_node_reads("\\p{WSpace}", true), "\\p{White_Space}");
        assert_eq!(as_node_reads("\\p{WSpace}", false), "\\p{WSpace}");
    }
}
