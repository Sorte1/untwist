use std::path::PathBuf;

use crate::obf_options::machinery::{measure, Found, MACHINERY, OUTPUT_ONLY};

use super::manifest::Cell;
use super::observe::{Observation, Ran};

pub const CHECKS: [&str; 4] = ["recognised", "modules", "clean", "node"];

pub fn measure_files(files: &[PathBuf]) -> Found {
    let mut total = Found {
        parses: true,
        ..Found::default()
    };
    for file in files {
        let text = std::fs::read(file)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        let found = measure(&text);
        total.parses &= found.parses;
        for (class, n) in found.counts {
            *total.counts.entry(class).or_default() += n;
        }
    }
    total
}

fn classes() -> Vec<&'static str> {
    MACHINERY
        .iter()
        .copied()
        .filter(|c| !OUTPUT_ONLY.contains(c))
        .collect()
}

pub fn left_over(output: &Found, base: &Found) -> Vec<String> {
    classes()
        .into_iter()
        .filter(|c| output.get(c) > base.get(c))
        .map(|c| format!("{c}={}", output.get(c)))
        .collect()
}

fn recognised(cell: &Cell, o: &Observation) -> Option<String> {
    match &o.bundle {
        Some((kind, _)) if *kind == cell.family => None,
        Some((kind, n)) => Some(format!("unpacked as {kind}:{n}, want {}", cell.family)),
        None => Some(format!("not unpacked, want {}", cell.family)),
    }
}

fn modules(cell: &Cell, o: &Observation) -> Option<String> {
    let (want, (_, got)) = (cell.source_modules?, o.bundle.as_ref()?);
    (want != *got).then(|| format!("{got} modules, the bundler read {want}"))
}

fn clean(o: &Observation, base: &Found) -> Option<String> {
    let found = &o.machinery;
    if !found.parses {
        return Some("an output file does not parse".to_string());
    }
    let left = left_over(found, base);
    (!left.is_empty()).then(|| format!("machinery left: {}", left.join(",")))
}

fn node(o: &Observation, expected: &Ran) -> Option<String> {
    let ran = o.ran.as_ref()?;
    (ran != expected).then(|| {
        let first = ran
            .render()
            .lines()
            .zip(expected.render().lines())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        format!(
            "output differs from the bundle at line {}: {:?}",
            first + 1,
            ran.render().lines().nth(first).unwrap_or("")
        )
    })
}

pub fn evaluate(
    cell: &Cell,
    o: &Observation,
    expected: &Ran,
    base: &Found,
) -> Vec<(&'static str, Option<String>)> {
    if let Some(error) = &o.untwist_error {
        return CHECKS.iter().map(|&c| (c, Some(error.clone()))).collect();
    }
    vec![
        ("recognised", recognised(cell, o)),
        ("modules", modules(cell, o)),
        ("clean", clean(o, base)),
        ("node", node(o, expected)),
    ]
}

pub fn parse_expected(text: &str) -> Ran {
    let (head, stdout) = text.split_once('\n').unwrap_or((text, ""));
    let exit = head.strip_prefix("exit=").and_then(|e| e.parse().ok());
    Ran {
        exit,
        stdout: stdout.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_expected_file_round_trips_through_render() {
        let ran = Ran {
            exit: Some(0),
            stdout: "a\nb\n".to_string(),
        };
        assert_eq!(parse_expected(&ran.render()), ran);
    }

    #[test]
    fn only_machinery_beyond_the_base_counts_as_left_over() {
        let mut base = Found::default();
        base.counts.insert("global_getters", 2);
        let mut out = base.clone();
        out.counts.insert("string_arrays", 1);
        out.counts.insert("dead_declarations", 9);
        assert_eq!(left_over(&out, &base), ["string_arrays=1"]);
    }
}
