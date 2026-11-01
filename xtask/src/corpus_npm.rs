use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const DEFAULT_SEED: u64 = 0x6a73_6372;
const INPUT_PROGRAM: &str =
    "function greet() {\n  return \"Hello World!\";\n}\nconsole.log(greet());\n";
const EXPECTED_OUTPUT: &str = "Hello World!";

struct Factor {
    name: &'static str,
    flag: &'static str,
    values: &'static [&'static str],
}

const FACTORS: &[Factor] = &[
    Factor {
        name: "stringArrayEncoding",
        flag: "--string-array-encoding",
        values: &["none", "base64", "rc4"],
    },
    Factor {
        name: "rotateStringArray",
        flag: "--string-array-rotate",
        values: &["false", "true"],
    },
    Factor {
        name: "shuffleStringArray",
        flag: "--string-array-shuffle",
        values: &["false", "true"],
    },
    Factor {
        name: "stringArrayIndexShift",
        flag: "--string-array-index-shift",
        values: &["false", "true"],
    },
    Factor {
        name: "stringArrayIndexesType",
        flag: "--string-array-indexes-type",
        values: &["hexadecimal-number", "hexadecimal-numeric-string"],
    },
    Factor {
        name: "stringArrayWrappersType",
        flag: "--string-array-wrappers-type",
        values: &["variable", "function"],
    },
    Factor {
        name: "stringArrayWrappersChainedCalls",
        flag: "--string-array-wrappers-chained-calls",
        values: &["false", "true"],
    },
    Factor {
        name: "stringArrayWrappersParametersMaxCount",
        flag: "--string-array-wrappers-parameters-max-count",
        values: &["2", "4"],
    },
    Factor {
        name: "stringArrayCallsTransform",
        flag: "--string-array-calls-transform",
        values: &["false", "true"],
    },
    Factor {
        name: "splitStrings",
        flag: "--split-strings",
        values: &["false", "true"],
    },
    Factor {
        name: "numbersToExpressions",
        flag: "--numbers-to-expressions",
        values: &["false", "true"],
    },
    Factor {
        name: "controlFlowFlattening",
        flag: "--control-flow-flattening",
        values: &["false", "true"],
    },
    Factor {
        name: "deadCodeInjection",
        flag: "--dead-code-injection",
        values: &["false", "true"],
    },
    Factor {
        name: "transformObjectKeys",
        flag: "--transform-object-keys",
        values: &["false", "true"],
    },
    Factor {
        name: "selfDefending",
        flag: "--self-defending",
        values: &["false", "true"],
    },
    Factor {
        name: "debugProtection",
        flag: "--debug-protection",
        values: &["false", "true"],
    },
];

type Case = Vec<usize>;
type Pair = (usize, usize, usize, usize);

struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn index(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

fn all_pairs() -> Vec<Pair> {
    let mut pairs = Vec::new();
    for (a, factor_a) in FACTORS.iter().enumerate() {
        for (b, factor_b) in FACTORS.iter().enumerate().skip(a + 1) {
            for va in 0..factor_a.values.len() {
                for vb in 0..factor_b.values.len() {
                    pairs.push((a, va, b, vb));
                }
            }
        }
    }
    pairs
}

fn covers(case: &[usize], pair: Pair) -> bool {
    let (a, va, b, vb) = pair;
    case[a] == va && case[b] == vb
}

fn build_case(seed_pair: Pair, needed: &[Pair], rng: &mut SplitMix64) -> Case {
    let mut assigned: Vec<Option<usize>> = vec![None; FACTORS.len()];
    assigned[seed_pair.0] = Some(seed_pair.1);
    assigned[seed_pair.2] = Some(seed_pair.3);
    for factor in 0..FACTORS.len() {
        if assigned[factor].is_some() {
            continue;
        }
        let values = FACTORS[factor].values;
        let mut scores = vec![0usize; values.len()];
        for (index, score) in scores.iter_mut().enumerate() {
            *score = needed
                .iter()
                .filter(|&&(a, va, b, vb)| {
                    (a == factor && va == index && assigned[b] == Some(vb))
                        || (b == factor && vb == index && assigned[a] == Some(va))
                })
                .count();
        }
        let best_score = scores.iter().copied().max().unwrap_or(0);
        let best: Vec<usize> = scores
            .iter()
            .enumerate()
            .filter(|&(_, &score)| score == best_score)
            .map(|(index, _)| index)
            .collect();
        let choice = best[rng.index(best.len())];
        assigned[factor] = Some(choice);
    }
    assigned
        .into_iter()
        .map(|value| value.unwrap_or(0))
        .collect()
}

fn pairwise_cover(seed: u64) -> Vec<Case> {
    let mut needed = all_pairs();
    let mut rng = SplitMix64::new(seed);
    let mut cases = Vec::new();
    while let Some(&seed_pair) = needed.first() {
        let case = build_case(seed_pair, &needed, &mut rng);
        needed.retain(|&pair| !covers(&case, pair));
        cases.push(case);
    }
    cases
}

fn ensure_installed(repo_root: &Path) -> Result<()> {
    let corpus_dir = repo_root.join("tools/corpus");
    let status = Command::new("npm")
        .arg("ci")
        .arg("--prefix")
        .arg(&corpus_dir)
        .status()
        .with_context(|| format!("spawn npm ci --prefix {}", corpus_dir.display()))?;
    if !status.success() {
        bail!(
            "npm ci --prefix {} exited with {status}",
            corpus_dir.display()
        );
    }
    Ok(())
}

fn case_options(case: &[usize]) -> serde_json::Map<String, serde_json::Value> {
    let mut options = serde_json::Map::new();
    for (factor, &value_index) in case.iter().enumerate() {
        let factor = &FACTORS[factor];
        options.insert(
            factor.name.to_string(),
            serde_json::json!(factor.values[value_index]),
        );
    }
    options
}

fn run_case(
    binary: &Path,
    input: &Path,
    output_dir: &Path,
    seed: u64,
    index: usize,
    case: &[usize],
) -> Result<serde_json::Value> {
    let name = format!("case-{index:03}.js");
    let output_path = output_dir.join(&name);
    let mut command = Command::new(binary);
    command
        .arg(input)
        .arg("--output")
        .arg(&output_path)
        .arg("--string-array")
        .arg("true")
        .arg("--string-array-threshold")
        .arg("1")
        .arg("--seed")
        .arg(seed.to_string());
    for (factor, &value_index) in case.iter().enumerate() {
        let factor = &FACTORS[factor];
        command.arg(factor.flag).arg(factor.values[value_index]);
    }
    let out = command
        .output()
        .with_context(|| format!("spawn {}", binary.display()))?;
    if !out.status.success() {
        bail!(
            "{} failed on {name}: {}",
            binary.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(serde_json::json!({
        "file": name,
        "options": case_options(case),
        "expected_output": EXPECTED_OUTPUT,
    }))
}

fn write_input() -> Result<tempfile::TempDir> {
    let input_dir = tempfile::tempdir().context("BLOCKED: create temp input dir")?;
    std::fs::write(input_dir.path().join("input.js"), INPUT_PROGRAM)
        .context("BLOCKED: write corpus input")?;
    Ok(input_dir)
}

fn prepare_output_dir(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir).context("BLOCKED: clear fixtures/corpus-npm")?;
    }
    std::fs::create_dir_all(output_dir).context("BLOCKED: create fixtures/corpus-npm")
}

fn run_all_cases(
    binary: &Path,
    input_path: &Path,
    output_dir: &Path,
    seed: u64,
    cases: &[Case],
) -> Result<Vec<serde_json::Value>> {
    let mut manifest = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        match run_case(binary, input_path, output_dir, seed, index, case) {
            Ok(entry) => manifest.push(entry),
            Err(err) => {
                let _ = std::fs::remove_dir_all(output_dir);
                return Err(err.context("BLOCKED: javascript-obfuscator failed"));
            }
        }
    }
    Ok(manifest)
}

fn write_manifest(output_dir: &Path, manifest: &[serde_json::Value]) -> Result<()> {
    let rendered = serde_json::to_string_pretty(manifest).context("BLOCKED: render manifest")?;
    std::fs::write(output_dir.join("manifest.json"), rendered).map_err(|err| {
        let _ = std::fs::remove_dir_all(output_dir);
        anyhow::Error::new(err).context("BLOCKED: write manifest.json")
    })
}

pub fn generate(repo_root: &Path, seed: u64) -> Result<usize> {
    ensure_installed(repo_root).context("BLOCKED: install javascript-obfuscator")?;
    let cases = pairwise_cover(seed);
    let input_dir = write_input()?;
    let input_path = input_dir.path().join("input.js");
    let output_dir = repo_root.join("fixtures/corpus-npm");
    prepare_output_dir(&output_dir)?;
    let binary = repo_root.join("tools/corpus/node_modules/.bin/javascript-obfuscator");
    let manifest = run_all_cases(&binary, &input_path, &output_dir, seed, &cases)?;
    write_manifest(&output_dir, &manifest)?;
    Ok(manifest.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_produces_identical_case_lists() {
        let a = pairwise_cover(DEFAULT_SEED);
        let b = pairwise_cover(DEFAULT_SEED);
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_can_produce_different_case_lists() {
        let a = pairwise_cover(DEFAULT_SEED);
        let b = pairwise_cover(DEFAULT_SEED.wrapping_add(1));
        assert_ne!(a, b);
    }

    #[test]
    fn the_cover_is_a_few_dozen_cases_and_covers_every_pair() {
        let cases = pairwise_cover(DEFAULT_SEED);
        assert!(cases.len() >= 8, "cover too small: {}", cases.len());
        assert!(cases.len() <= 100, "cover too large: {}", cases.len());
        for pair in all_pairs() {
            assert!(
                cases.iter().any(|case| covers(case, pair)),
                "pair {pair:?} never covered"
            );
        }
    }
}
