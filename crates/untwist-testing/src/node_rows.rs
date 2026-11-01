use std::path::{Path, PathBuf};

use crate::node::{node, run, skip_allowed, Observed, SKIP_MESSAGE};

pub fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

pub fn as_script(text: &str) -> String {
    let quoted = serde_json::to_string(text).unwrap();
    format!(
        "try {{ require('vm').runInThisContext({quoted}); }} catch (e) {{ console.log('threw ' + e.constructor.name); }}"
    )
}

pub fn observed(o: &Observed) -> String {
    let exit = o
        .exit
        .map_or("no exit code".to_string(), |code| code.to_string());
    format!("{}exit {exit}\n", o.stdout)
}

pub fn printed(node: &Path, dir: &Path, text: &str) -> Result<String, String> {
    let path = dir.join("program.js");
    let mut seen = String::new();
    for form in [text.to_string(), as_script(text)] {
        std::fs::write(&path, form).unwrap();
        let out = run(node, &path).unwrap();
        let Some(out) = out else {
            return Err(format!("did not finish; printed so far {seen:?}"));
        };
        seen.push_str(&observed(&out));
        seen.push_str("--\n");
    }
    Ok(seen)
}

pub type Mode<'m> = (&'static str, &'m dyn Fn(&str) -> String);

pub fn mismatches(node: &Path, programs: &[(String, String)], modes: &[Mode<'_>]) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for (label, source) in programs {
        let before = printed(node, dir.path(), source);
        for (mode, transform) in modes {
            let after = printed(node, dir.path(), &transform(source));
            if before.is_err() || after.is_err() || after != before {
                wrong.push(format!(
                    "{label} ({mode}): node printed {before:?}, untwist's output printed {after:?}"
                ));
            }
        }
    }
    wrong
}

const DYNAMIC_CODE: [&str; 4] = ["eval", "with", "Function", "constructor"];

pub fn runs_dynamic_code(program: &str) -> bool {
    DYNAMIC_CODE.iter().any(|marker| program.contains(marker))
}

pub fn unsafe_dynamic_code_witnesses(
    wrong: Vec<String>,
    programs: &[(String, String)],
) -> (Vec<String>, Vec<String>) {
    let mut unexplained = Vec::new();
    let mut witnesses = Vec::new();
    for row in wrong {
        let witness = programs.iter().find(|(label, program)| {
            row.starts_with(&format!("{label} (unsafe):")) && runs_dynamic_code(program)
        });
        match witness {
            Some((label, _)) => witnesses.push(label.clone()),
            None => unexplained.push(row),
        }
    }
    (unexplained, witnesses)
}

pub fn listed_unsafe_witnesses(wrong: Vec<String>, labels: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut unexplained = Vec::new();
    let mut witnesses = Vec::new();
    for row in wrong {
        let listed = labels
            .iter()
            .find(|label| row.starts_with(&format!("{label} (unsafe):")));
        match listed {
            Some(label) => witnesses.push(label.to_string()),
            None => unexplained.push(row),
        }
    }
    (unexplained, witnesses)
}

pub fn machinery_prefix(source: &str) -> Option<&str> {
    let start = source.find("})(a")?;
    let tail = ", 0);";
    let end = start + source[start..].find(tail)? + tail.len();
    Some(&source[..end])
}

fn first_function_name(text: &str) -> Option<&str> {
    let rest = &text[text.find("function ")? + "function ".len()..];
    rest.split('(').next().map(str::trim)
}

pub fn with_machinery_restored(source: &str, output: &str) -> String {
    let Some(prefix) = machinery_prefix(source) else {
        return output.to_string();
    };
    let kept = first_function_name(prefix)
        .is_some_and(|name| output.contains(&format!("function {name}(")));
    if kept {
        return output.to_string();
    }
    format!("{prefix}\n{output}")
}

pub fn patched_after_rotator(test: &str, patch: &str) -> String {
    patched_after_named_rotator("", test, patch)
}

pub fn patched_after_named_rotator(suffix: &str, test: &str, patch: &str) -> String {
    format!(
        "function a{suffix}() {{ var h = ['Hello World!', 'console', 'deobfuscate', 'log']; a{suffix} = function () {{ return h; }}; return a{suffix}(); }}
function b{suffix}(c, d) {{
var e = a{suffix}();
return (b{suffix} = function (f, g) {{
f = f - 0;
var i = e[f];
return i + ({test} ? '!' : '?');
}}), b{suffix}(c, d);
}}
(function (c, d) {{
var e = c();
var tries = 0;
while (!![]) {{
try {{
if (b{suffix}(0) === 'log!') break;
else e['push'](e['shift']());
}} catch (h) {{ e['push'](e['shift']()); }}
if (tries++ > 1000) break;
}}
}})(a{suffix}, 0);
{patch}
console.log(b{suffix}(0));
console.log(b{suffix}(1));
console.log(b{suffix}(3));
"
    )
}
