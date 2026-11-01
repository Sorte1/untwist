use std::path::{Path, PathBuf};

use untwist_testing::depth::{print_estimate, scan, DepthScan, C_P1, UNLIMITED};
use untwist_testing::depth_classes::{generate, ALL};

use super::harness::{argv, codes, untwist, workspace_file, Rows, Scratch};
use super::refusal::{refusal_depths, unparsable, FLAG_SETS};

const LADDER_FIRST: (&str, &str) = ("UNTWIST_TEST_LADDER_FIRST", "67108864");
const SWEEP_DEPTHS: [usize; 2] = [1_000, 10_000];
const DEEP_INPUT: u32 = 1_000;
const COMMA_LOGICAL_DEPTH: usize = 200;
const FIXTURE_DIRS: [&str; 8] = [
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/unpack-gaps",
    "fixtures/deobfuscate-gaps",
    "fixtures/eval",
    "fixtures/behaviour",
    "fixtures/jsx",
    "fixtures/unminify",
];

pub fn no_class_kills_a_sixty_four_mib_rung(rows: &mut Rows, scratch: &Scratch) {
    for &class in ALL {
        for depth in refusal_depths(class) {
            let input = scratch.file("w2.js", &generate(class, depth));
            for flags in FLAG_SETS {
                let run = untwist(&argv(flags, &[&input]), &[LADDER_FIRST]);
                let allowed = matches!(run.code(), Some(0 | 3))
                    || (unparsable(class) && run.code() == Some(1));
                let label = format!("{} at {depth} {flags:?}", class.name());
                rows.check(&label, allowed && !run.died(), || run.summary());
            }
        }
    }
}

#[derive(Default)]
struct Calibration {
    inputs: usize,
    largest_excess: i64,
    excess_input: String,
    largest_affine_excess: i64,
    largest_factor: f64,
    factor_input: String,
    largest_print_share: f64,
    print_input: String,
}

impl Calibration {
    fn record(&mut self, label: &str, input: &DepthScan, output: &DepthScan, shares: (u64, u64)) {
        self.inputs += 1;
        let excess = i64::from(output.nesting) - i64::from(input.nesting);
        if excess > self.largest_excess {
            self.largest_excess = excess;
            self.excess_input = label.to_string();
        }
        let affine = i64::from(output.nesting) - 2 * i64::from(input.nesting);
        self.largest_affine_excess = self.largest_affine_excess.max(affine);
        if input.nesting >= DEEP_INPUT {
            let factor = f64::from(output.nesting) / f64::from(input.nesting);
            if factor > self.largest_factor {
                self.largest_factor = factor;
                self.factor_input = label.to_string();
            }
        }
        let share = shares.0 as f64 / shares.1.max(1) as f64;
        if share > self.largest_print_share {
            self.largest_print_share = share;
            self.print_input = label.to_string();
        }
    }

    fn print(&self) {
        println!(
            "calibration over {} runs: largest nesting excess {} ({}), largest excess \
             over 2x {} (C_P1 {C_P1}), largest factor at nesting >= {DEEP_INPUT} {:.3} ({}), \
             largest output share of the print estimate {:.4} ({})",
            self.inputs,
            self.largest_excess,
            self.excess_input,
            self.largest_affine_excess,
            self.largest_factor,
            self.factor_input,
            self.largest_print_share,
            self.print_input
        );
    }
}

fn premise_holds(input: &DepthScan, output: &DepthScan) -> bool {
    u64::from(output.nesting) <= 2 * u64::from(input.nesting) + u64::from(C_P1)
        && output.speculative_parens <= input.speculative_parens.max(1)
        && output.speculative_pairs <= input.speculative_pairs
}

fn sweep_one(rows: &mut Rows, calibration: &mut Calibration, label: &str, input: &Path) {
    let source = std::fs::read_to_string(input).unwrap_or_default();
    let Ok(before) = scan(&source, &UNLIMITED) else {
        return;
    };
    for flags in FLAG_SETS {
        let mut all = vec!["--report", "json"];
        all.extend_from_slice(flags);
        let run = untwist(&argv(&all, &[input]), &[]);
        if run.code() != Some(0) {
            rows.check(&format!("{label} {flags:?} runs"), !run.died(), || {
                run.summary()
            });
            continue;
        }
        let text = codes(&run.json()).concat();
        let Ok(after) = scan(&text, &UNLIMITED) else {
            rows.check(&format!("{label} {flags:?} scans"), false, || {
                "the final text does not scan".to_string()
            });
            continue;
        };
        let estimate = print_estimate(&before, source.len());
        let printed = text.len() as u64;
        let name = format!("{label} {flags:?}");
        calibration.record(&name, &before, &after, (printed, estimate));
        rows.check(&format!("{name}"), premise_holds(&before, &after), || {
            format!("input {before:?} final {after:?}")
        });
        rows.check(&format!("{name}"), printed <= estimate, || {
            format!("printed {printed} B, estimate {estimate} B")
        });
    }
}

fn fixture_inputs() -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = untwist_testing::real_world_path("game.js")
        .into_iter()
        .collect();
    for dir in FIXTURE_DIRS {
        let path = workspace_file(dir);
        let mut found: Vec<PathBuf> = std::fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
            .collect();
        found.sort();
        inputs.extend(found);
    }
    inputs
}

pub fn premise_and_print_estimate(rows: &mut Rows, scratch: &Scratch) {
    let mut calibration = Calibration::default();
    for &class in ALL {
        for depth in SWEEP_DEPTHS {
            let input = scratch.file("w3.js", &generate(class, depth));
            let label = format!("{} at {depth}", class.name());
            sweep_one(rows, &mut calibration, &label, &input);
        }
    }
    let comma_logicals = format!(
        "{}a&&b,c&&d,e&&f,g&&h;{}",
        "function f(){".repeat(COMMA_LOGICAL_DEPTH),
        "}".repeat(COMMA_LOGICAL_DEPTH)
    );
    let input = scratch.file("w4-comma-logicals.js", &comma_logicals);
    let label = format!("a&&b,c&&d at depth {COMMA_LOGICAL_DEPTH}");
    sweep_one(rows, &mut calibration, &label, &input);
    for input in fixture_inputs() {
        let label = input
            .strip_prefix(workspace_file(""))
            .unwrap_or(&input)
            .display()
            .to_string();
        sweep_one(rows, &mut calibration, &label, &input);
    }
    calibration.print();
}
