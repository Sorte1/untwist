use std::str::SplitWhitespace;

use untwist::{Options, RenameMode};

const VALUE_FLAGS: [&str; 6] = [
    "--rename",
    "--max-iterations",
    "--fuel",
    "-j",
    "--jobs",
    "--disable",
];

fn value<'f>(words: &mut SplitWhitespace<'f>, flag: &str) -> Result<&'f str, String> {
    words.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn rename_mode(word: &str) -> Result<RenameMode, String> {
    match word {
        "none" => Ok(RenameMode::None),
        "mangle" => Ok(RenameMode::Mangle),
        "simple" => Ok(RenameMode::Simple),
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
        _ => return None,
    }
    Some(options)
}

fn valued(mut options: Options, flag: &str, word: &str) -> Result<Options, String> {
    match flag {
        "--rename" => options.rename = rename_mode(word)?,
        "--max-iterations" => options.max_iterations = Some(number(word, flag)?),
        "--fuel" => options.fuel = Some(fuel(word)?),
        "--disable" => return disable(options, word),
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
    let mut options = Options::default();
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
