use std::path::PathBuf;

pub const PEAK_EXPANSION: u64 = 75;
pub const RETAINED_PER_INPUT_BYTE: u64 = 2;
pub const UNPACK_PER_INPUT_BYTE: u64 = 27;
const MIB: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub width: usize,
    pub unpack: bool,
}

pub struct Budget {
    pub input_sizes: Vec<u64>,
    pub available_bytes: u64,
}

pub fn budget(paths: &[PathBuf]) -> Option<Budget> {
    if paths.is_empty() {
        return None;
    }
    let input_sizes = paths
        .iter()
        .filter_map(|path| std::fs::metadata(path).ok())
        .map(|meta| meta.len())
        .collect();
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    Some(Budget {
        input_sizes,
        available_bytes: system.available_memory(),
    })
}

fn saturating_sum<'s>(sizes: impl Iterator<Item = &'s u64>) -> u64 {
    sizes.copied().fold(0, u64::saturating_add)
}

pub fn needed_bytes(sizes: &[u64], shape: Shape) -> u64 {
    let mut largest = sizes.to_vec();
    largest.sort_unstable_by(|a, b| b.cmp(a));
    let width = shape.width.max(1);
    let live = saturating_sum(largest.iter().take(width));
    let rest = saturating_sum(largest.iter().skip(width));
    let stages = live
        .saturating_mul(PEAK_EXPANSION)
        .saturating_add(rest.saturating_mul(RETAINED_PER_INPUT_BYTE));
    if !shape.unpack {
        return stages;
    }
    let total = saturating_sum(sizes.iter());
    stages.max(total.saturating_mul(UNPACK_PER_INPUT_BYTE))
}

fn unpack_clause(shape: Shape) -> String {
    if !shape.unpack {
        return String::new();
    }
    format!(", and about {UNPACK_PER_INPUT_BYTE}x all of them together while unpacking")
}

pub fn warning(budget: &Budget, cap_mb: Option<u64>, shape: Shape) -> Option<String> {
    let input_bytes = saturating_sum(budget.input_sizes.iter());
    let input_mb = input_bytes.div_ceil(MIB);
    if let Some(cap) = cap_mb {
        if input_bytes > cap.saturating_mul(MIB) {
            return Some(format!(
                "input is {input_mb} MiB, over the --max-input-mb {cap} you asked for; continuing"
            ));
        }
        return None;
    }
    if budget.available_bytes == 0 {
        return None;
    }
    let needed = needed_bytes(&budget.input_sizes, shape);
    if needed <= budget.available_bytes {
        return None;
    }
    Some(format!(
        "input is {input_mb} MiB and untwist has peaked at about {PEAK_EXPANSION}x each input it \
         holds at once ({} at a time){}, so it may need {} MiB against {} MiB available; \
         continuing, but the run may be killed",
        shape.width,
        unpack_clause(shape),
        needed.div_ceil(MIB),
        budget.available_bytes.div_ceil(MIB)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    fn stages(width: usize) -> Shape {
        Shape {
            width,
            unpack: false,
        }
    }

    fn budget_of(input_mb: u64, available_mb: u64) -> Budget {
        Budget {
            input_sizes: vec![input_mb * MIB],
            available_bytes: available_mb * 1024 * 1024,
        }
    }

    #[test]
    fn a_small_input_on_a_large_machine_says_nothing() {
        assert_eq!(warning(&budget_of(10, 16_000), None, stages(1)), None);
    }

    #[test]
    fn seventy_five_times_the_input_over_available_memory_warns() {
        let text = warning(&budget_of(900, 8_000), None, stages(1)).expect("a warning");
        assert!(text.contains("900 MiB"), "{text}");
        assert!(text.contains("75x"), "{text}");
        assert!(text.contains("67500 MiB"), "{text}");
        assert!(text.contains("8000 MiB"), "{text}");
    }

    #[test]
    fn the_explicit_cap_wins_over_the_memory_estimate() {
        let text = warning(&budget_of(10, 16_000), Some(5), stages(1)).expect("a warning");
        assert!(text.contains("--max-input-mb 5"), "{text}");
    }

    #[test]
    fn an_input_inside_the_explicit_cap_says_nothing() {
        assert_eq!(warning(&budget_of(4, 16_000), Some(5), stages(1)), None);
    }

    #[test]
    fn zero_available_memory_is_treated_as_unknown_and_never_warns_on_its_own() {
        assert_eq!(
            warning(
                &Budget {
                    input_sizes: vec![1024],
                    available_bytes: 0
                },
                None,
                stages(1)
            ),
            None
        );
    }

    #[test]
    fn only_the_inputs_a_run_holds_at_once_are_multiplied() {
        let sizes = [100 * MIB, 10 * MIB, 10 * MIB];
        assert_eq!(needed_bytes(&sizes, stages(1)), (100 * 75 + 20 * 2) * MIB);
        assert_eq!(needed_bytes(&sizes, stages(2)), (110 * 75 + 10 * 2) * MIB);
        assert_eq!(needed_bytes(&sizes, stages(3)), 120 * 75 * MIB);
    }

    #[test]
    fn one_input_needs_seventy_five_times_itself_at_any_width() {
        assert_eq!(needed_bytes(&[4 * MIB], stages(1)), 300 * MIB);
        assert_eq!(needed_bytes(&[4 * MIB], stages(8)), 300 * MIB);
    }

    #[test]
    fn the_unpack_arena_holds_every_input_at_once() {
        let sizes = [100 * MIB; 10];
        let unpacking = Shape {
            width: 1,
            unpack: true,
        };
        assert_eq!(needed_bytes(&sizes, stages(1)), (100 * 75 + 900 * 2) * MIB);
        assert_eq!(needed_bytes(&sizes, unpacking), 1000 * 27 * MIB);
        assert_eq!(needed_bytes(&[4 * MIB], unpacking), 300 * MIB);
    }

    #[test]
    fn many_small_inputs_under_one_job_warn_only_when_unpack_needs_them_all() {
        let budget = Budget {
            input_sizes: vec![100 * MIB; 10],
            available_bytes: 16_000 * MIB,
        };
        assert_eq!(warning(&budget, None, stages(1)), None);
        assert!(warning(&budget, None, stages(10)).is_some());
        let unpacking = Shape {
            width: 1,
            unpack: true,
        };
        let text = warning(&budget, None, unpacking).expect("a warning");
        assert!(text.contains("27x"), "{text}");
    }

    #[test]
    fn sizes_that_overflow_a_u64_saturate_instead_of_wrapping() {
        let huge = vec![u64::MAX / 2; 5];
        let unpacking = Shape {
            width: 2,
            unpack: true,
        };
        assert_eq!(needed_bytes(&huge, stages(2)), u64::MAX);
        assert_eq!(needed_bytes(&huge, stages(5)), u64::MAX);
        assert_eq!(needed_bytes(&huge, unpacking), u64::MAX);
        let budget = Budget {
            input_sizes: huge,
            available_bytes: 16_000 * MIB,
        };
        let text = warning(&budget, Some(5), stages(1)).expect("a warning");
        assert!(
            text.contains(&format!("{} MiB", u64::MAX.div_ceil(MIB))),
            "{text}"
        );
    }
}
