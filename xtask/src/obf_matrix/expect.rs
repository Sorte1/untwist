use std::path::Path;

use anyhow::{bail, Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    ExpectedFail,
    Allowed,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub fixture: String,
    pub cell: String,
    pub check: String,
    pub mark: Mark,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail(String),
    ExpectedFail(String),
    UnexpectedPass(String),
    Allowed(String),
}

impl Verdict {
    pub fn is_failure(&self) -> bool {
        matches!(self, Verdict::Fail(_) | Verdict::UnexpectedPass(_))
    }

    pub fn tag(&self) -> &'static str {
        match self {
            Verdict::Pass => "ok",
            Verdict::Fail(_) => "FAIL",
            Verdict::ExpectedFail(_) => "XFAIL",
            Verdict::UnexpectedPass(_) => "XPASS",
            Verdict::Allowed(_) => "ALLOW",
        }
    }
}

pub fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let mut rest = text;
    for (index, part) in parts.iter().enumerate() {
        if index == 0 {
            let Some(after) = rest.strip_prefix(part) else {
                return false;
            };
            rest = after;
        } else if index == parts.len() - 1 {
            return rest.ends_with(part);
        } else if let Some(at) = rest.find(part) {
            rest = &rest[at + part.len()..];
        } else {
            return false;
        }
    }
    true
}

pub fn parse(text: &str) -> Result<Vec<Rule>> {
    let mut rules = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [fixture, cell, check, mark, reason] = fields.as_slice() else {
            bail!(
                "line {}: want 5 tab-separated fields, got {}",
                number + 1,
                fields.len()
            );
        };
        let mark = match *mark {
            "xfail" => Mark::ExpectedFail,
            "allow" => Mark::Allowed,
            other => bail!(
                "line {}: mark {other:?} is neither xfail nor allow",
                number + 1
            ),
        };
        if reason.trim().is_empty() {
            bail!("line {}: a mark needs a reason", number + 1);
        }
        rules.push(Rule {
            fixture: fixture.to_string(),
            cell: cell.to_string(),
            check: check.to_string(),
            mark,
            reason: reason.to_string(),
        });
    }
    Ok(rules)
}

pub fn load(path: &Path) -> Result<Vec<Rule>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    parse(&std::fs::read_to_string(path).with_context(|| path.display().to_string())?)
}

pub fn judge(
    rules: &[Rule],
    fixture: &str,
    cell: &str,
    check: &str,
    failure: Option<String>,
) -> Verdict {
    let rule = rules
        .iter()
        .find(|r| r.check == check && glob(&r.fixture, fixture) && glob(&r.cell, cell));
    match (rule, failure) {
        (None, None) => Verdict::Pass,
        (None, Some(detail)) => Verdict::Fail(detail),
        (Some(rule), Some(_)) if rule.mark == Mark::ExpectedFail => {
            Verdict::ExpectedFail(rule.reason.clone())
        }
        (Some(rule), Some(_)) => Verdict::Allowed(rule.reason.clone()),
        (Some(rule), None) if rule.mark == Mark::ExpectedFail => {
            Verdict::UnexpectedPass(rule.reason.clone())
        }
        (Some(_), None) => Verdict::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_prefixes_suffixes_and_middles() {
        assert!(glob("*", "anything"));
        assert!(glob("unsafe/*/dir", "unsafe/simple/dir"));
        assert!(!glob("unsafe/*/dir", "safe/simple/dir"));
        assert!(glob("game-*-7.1.8.js", "game-ab12-7.1.8.js"));
        assert!(!glob("a", "ab"));
    }

    #[test]
    fn an_expected_failure_that_passes_is_a_failure() {
        let rules = parse("x.js\t*/simple/*\trename\txfail\tsimple-rename-gap\n").unwrap();
        assert_eq!(
            judge(
                &rules,
                "x.js",
                "safe/simple/stdout",
                "rename",
                Some("d".into())
            ),
            Verdict::ExpectedFail("simple-rename-gap".into())
        );
        assert!(judge(&rules, "x.js", "safe/simple/stdout", "rename", None).is_failure());
        assert!(judge(
            &rules,
            "x.js",
            "safe/none/stdout",
            "rename",
            Some("d".into())
        )
        .is_failure());
    }

    #[test]
    fn a_rule_without_a_reason_is_rejected() {
        assert!(parse("x\ty\tz\txfail\t \n").is_err());
        assert!(parse("x\ty\tz\tmaybe\tr\n").is_err());
    }
}
