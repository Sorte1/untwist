use std::str::SplitWhitespace;

use untwist::{Options, RenameMode, WasmCheck};

pub const DEFAULT_MAX_MEMORY: u64 = 1 << 30;

const VALUE_FLAGS: [&str; 10] = [
    "--rename",
    "--modern",
    "--wasm-check",
    "--max-iterations",
    "--fuel",
    "-j",
    "--jobs",
    "--disable",
    "--max-memory",
    "--timeout",
];

fn value<'f>(words: &mut SplitWhitespace<'f>, flag: &str) -> Result<&'f str, String> {
    words.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn rename_mode(word: &str) -> Result<RenameMode, String> {
    match word {
        "none" => Ok(RenameMode::None),
        "mangle" => Ok(RenameMode::Mangle),
        "simple" => Ok(RenameMode::Simple),
        "smart" => Ok(RenameMode::Smart),
        "unique" => Ok(RenameMode::Unique),
        other => Err(format!("unknown rename mode {other}")),
    }
}

fn number<T: std::str::FromStr>(word: &str, flag: &str) -> Result<T, String> {
    word.parse()
        .map_err(|_| format!("{flag} takes a number, not {word}"))
}

fn fuel(word: &str) -> Result<u64, String> {
    if word == "unlimited" {
        return Ok(u64::MAX);
    }
    match word.parse::<u64>() {
        Ok(0) => Err("--fuel must be at least 1".to_string()),
        Ok(steps) => Ok(steps),
        Err(_) => Err(format!("--fuel takes a number or unlimited, not {word}")),
    }
}

fn disable(mut options: Options, word: &str) -> Result<Options, String> {
    for name in word.split(',') {
        let pass = untwist::pass_named(name)
            .ok_or_else(|| format!("unknown pass {name} (see --list-passes)"))?;
        options.disabled_passes.push(pass);
    }
    Ok(options)
}

fn switched(mut options: Options, flag: &str) -> Option<Options> {
    match flag {
        "--no-jsx" => options.jsx = false,
        "--no-unpack" => options.unpack = false,
        "--no-deobfuscate" => options.deobfuscate = false,
        "--no-unminify" => options.unminify = false,
        "--unsafe" => options.unsafe_transforms = true,
        "--keep-human-names" => options.rename_all = false,
        "--single-file" => options.single_file = true,
        "-m" | "--mangle" => options.rename = RenameMode::Mangle,
        "--source-map" => options.source_map = true,
        _ => return None,
    }
    Some(options)
}

fn valued(mut options: Options, flag: &str, word: &str) -> Result<Options, String> {
    match flag {
        "--rename" => options.rename = rename_mode(word)?,
        "--max-iterations" => options.max_iterations = Some(number(word, flag)?),
        "--fuel" => options.fuel = Some(fuel(word)?),
        "--max-memory" => {
            let bytes =
                untwist::budget_memory_size(word).map_err(|error| format!("{flag} {error}"))?;
            options.max_memory = (bytes != u64::MAX).then_some(bytes);
        }
        "--timeout" => {
            let timeout =
                untwist::budget_timeout_secs(word).map_err(|error| format!("{flag} {error}"))?;
            options.timeout = Some(timeout);
        }
        "--disable" => return disable(options, word),
        "--modern" => options.modern = Some(untwist::parse_modern_baseline(word)?),
        "--wasm-check" => {
            options.wasm_check = WasmCheck::named(word)
                .ok_or_else(|| format!("unknown wasm check {word} (off, on or auto)"))?;
        }
        "-j" | "--jobs" => {
            number::<std::num::NonZeroUsize>(word, flag)?;
        }
        other => return Err(format!("unsupported flag {other}")),
    }
    Ok(options)
}

fn apply(options: Options, flag: &str, words: &mut SplitWhitespace<'_>) -> Result<Options, String> {
    if VALUE_FLAGS.contains(&flag) {
        return valued(options, flag, value(words, flag)?);
    }
    switched(options, flag).ok_or_else(|| format!("unsupported flag {flag}"))
}

pub fn options_from(flags: &str) -> Result<Options, String> {
    let mut words = flags.split_whitespace();
    let mut options = Options {
        max_memory: Some(DEFAULT_MAX_MEMORY),
        ..Options::default()
    };
    while let Some(flag) = words.next() {
        options = apply(options, flag, &mut words)?;
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::options_from;
    use untwist::RenameMode;

    #[test]
    fn no_flags_are_the_clis_defaults() {
        let options = options_from("").unwrap();
        let defaults = untwist::Options::default();
        assert_eq!(
            (options.unpack, options.jsx, options.unsafe_transforms),
            (defaults.unpack, defaults.jsx, defaults.unsafe_transforms)
        );
    }

    #[test]
    fn the_memory_budget_has_a_playground_default_and_flags_change_it() {
        let options = options_from("").unwrap();
        assert_eq!(options.max_memory, Some(super::DEFAULT_MAX_MEMORY));
        assert_eq!(options.timeout, None);
        let options = options_from("--max-memory 256MiB --timeout 30").unwrap();
        assert_eq!(options.max_memory, Some(256 << 20));
        assert_eq!(options.timeout, Some(std::time::Duration::from_secs(30)));
        let options = options_from("--max-memory unlimited").unwrap();
        assert_eq!(options.max_memory, None);
        assert!(options_from("--max-memory lots").is_err());
    }

    #[test]
    fn modern_takes_a_baseline_without_spaces_and_is_off_by_default() {
        assert!(options_from("--unsafe").unwrap().modern.is_none());
        let options = options_from("--modern es2022").unwrap();
        assert_eq!(
            options
                .modern
                .map(|baseline| baseline.text().to_string())
                .as_deref(),
            Some("es2022")
        );
        let options = options_from("--modern chrome@120,firefox@121").unwrap();
        assert_eq!(
            options.modern.map(|baseline| baseline.engines().len()),
            Some(2)
        );
        assert!(options_from("--modern es1999").is_err());
    }

    #[test]
    fn the_switch_flags_set_their_options() {
        let options = options_from("--unsafe --no-unpack --no-jsx --rename unique -j 4").unwrap();
        assert!(options.unsafe_transforms);
        assert!(!options.unpack && !options.jsx);
        assert_eq!(options.rename, RenameMode::Unique);
    }

    #[test]
    fn a_value_flag_without_its_value_and_a_zero_fuel_are_refused() {
        assert_eq!(
            options_from("--rename").err().as_deref(),
            Some("--rename needs a value")
        );
        assert_eq!(
            options_from("--fuel 0").err().as_deref(),
            Some("--fuel must be at least 1")
        );
    }

    #[test]
    fn disable_names_passes_by_repeat_or_comma_and_refuses_an_unknown_one() {
        assert_eq!(
            options_from("--disable dead-code --disable yoda,sequence")
                .unwrap()
                .disabled_passes,
            vec!["dead-code", "yoda", "sequence"]
        );
        assert!(options_from("").unwrap().disabled_passes.is_empty());
        assert_eq!(
            options_from("--disable no-such-pass").err().as_deref(),
            Some("unknown pass no-such-pass (see --list-passes)")
        );
        assert_eq!(
            options_from("--disable").err().as_deref(),
            Some("--disable needs a value")
        );
    }

    #[test]
    fn wasm_check_defaults_to_off_and_takes_on_or_auto() {
        use untwist::WasmCheck;
        assert_eq!(options_from("").unwrap().wasm_check, WasmCheck::Off);
        assert_eq!(
            options_from("--wasm-check auto").unwrap().wasm_check,
            WasmCheck::Auto
        );
        assert_eq!(
            options_from("--wasm-check on").unwrap().wasm_check,
            WasmCheck::On
        );
        assert_eq!(
            options_from("--wasm-check maybe").err().as_deref(),
            Some("unknown wasm check maybe (off, on or auto)")
        );
    }

    #[test]
    fn an_unlimited_fuel_is_the_largest_budget() {
        assert_eq!(
            options_from("--fuel unlimited").unwrap().fuel,
            Some(u64::MAX)
        );
        assert_eq!(options_from("--fuel 7").unwrap().fuel, Some(7));
        assert_eq!(options_from("").unwrap().fuel, None);
        assert_eq!(
            options_from("--fuel lots").err().as_deref(),
            Some("--fuel takes a number or unlimited, not lots")
        );
    }
}
