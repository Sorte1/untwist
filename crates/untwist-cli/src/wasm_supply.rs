mod address;
mod fetch;
mod url;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use untwist::{printable, Output, SuppliedWasm, WasmOrigin, WasmReference, WasmSupply};

use crate::args::Args;
use crate::{hooks, STDERR};
use fetch::{Failure, Fetcher, LIMITS};

const MAX_WASM_BYTES: u64 = untwist::MAX_WASM_BYTES as u64;
const SUPPLY_PANICKED: &str = "supplying wasm modules hit an internal error; the output is \
     written without the modules --wasm, --wasm-dir and --fetch-wasm would give; please report it";

fn read_limited(path: &Path) -> io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_WASM_BYTES + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn supplied(name: String, bytes: Vec<u8>) -> SuppliedWasm {
    SuppliedWasm {
        name,
        bytes,
        origin: WasmOrigin::Supplied,
        url: None,
    }
}

fn dir_entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    paths.sort();
    Ok(paths)
}

pub fn supply(args: &Args, err: &mut dyn Write) -> anyhow::Result<WasmSupply> {
    let mut files = Vec::new();
    for spec in &args.wasm {
        let (name, path) = spec
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--wasm takes NAME=PATH, got {spec:?}"))?;
        let bytes = read_limited(Path::new(path))
            .map_err(|error| anyhow::anyhow!("read --wasm {path}: {error}"))?;
        files.push(supplied(name.to_string(), bytes));
    }
    if let Some(dir) = &args.wasm_dir {
        let paths = dir_entries(dir)
            .map_err(|error| anyhow::anyhow!("read --wasm-dir {}: {error}", dir.display()))?;
        for path in paths {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            match read_limited(&path) {
                Ok(bytes) => files.push(supplied(name.to_string(), bytes)),
                Err(error) => writeln!(
                    err,
                    "warning: {}",
                    printable(&format!("read {}: {error}", path.display()))
                )?,
            }
        }
    }
    Ok(WasmSupply { files })
}

pub fn fetch_url(reference: &WasmReference, base: Option<&str>) -> Result<String, String> {
    let target = &reference.url;
    if url::has_scheme(target) {
        return url::fetchable(target)
            .ok_or_else(|| format!("{target} is not an http(s) URL that can be requested"));
    }
    let Some(base) = base else {
        return Err(format!(
            "{target} is relative; pass --wasm-base to fetch it"
        ));
    };
    url::joined(base, target)
        .ok_or_else(|| format!("{target} gives no http(s) URL against --wasm-base {base}"))
}

pub fn fetch_unsupplied(
    args: &Args,
    result: &Output,
    supply: &mut WasmSupply,
    err: &mut dyn Write,
) -> io::Result<()> {
    if !args.fetch_wasm {
        return Ok(());
    }
    let limits = match hooks::fetch_request() {
        Some(request) => fetch::Limits { request, ..LIMITS },
        None => LIMITS,
    };
    let mut fetcher = Fetcher::new(args.fetch_wasm_local, limits);
    for reference in result.wasm_references() {
        if supply.find(&reference.name).is_some() {
            continue;
        }
        let name = printable(&reference.name);
        let unfetched = fetch_url(&reference, args.wasm_base.as_deref())
            .and_then(|target| fetcher.exhausted().map_or(Ok(target), Err));
        let target = match unfetched {
            Ok(target) => target,
            Err(why) => {
                writeln!(err, "note: not fetching {name}: {}", printable(&why))?;
                continue;
            }
        };
        match fetcher.fetch(&target, err) {
            Ok(bytes) => supply.files.push(SuppliedWasm {
                name: reference.name.clone(),
                bytes,
                origin: WasmOrigin::Fetched,
                url: Some(target),
            }),
            Err(Failure::Stderr(error)) => return Err(error),
            Err(Failure::Fetch(why)) => writeln!(
                err,
                "warning: fetching {} failed: {}",
                printable(&target),
                printable(&why)
            )?,
        }
    }
    Ok(())
}

fn supplied_and_fetched(
    args: &Args,
    result: &Output,
    err: &mut dyn Write,
) -> anyhow::Result<WasmSupply> {
    if hooks::wasm_panic() {
        panic!("wasm supply test hook");
    }
    let mut supply = supply(args, err)?;
    fetch_unsupplied(args, result, &mut supply, err).context(STDERR)?;
    Ok(supply)
}

pub fn settle(args: &Args, result: &mut Output, err: &mut dyn Write) -> anyhow::Result<()> {
    let supply = match untwist::contained(|| supplied_and_fetched(args, &*result, &mut *err)) {
        Some(supply) => supply?,
        None => {
            result.wasm_panicked = true;
            writeln!(err, "warning: {SUPPLY_PANICKED}").context(STDERR)?;
            WasmSupply::default()
        }
    };
    result.settle_wasm(&supply, args.output.is_some());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(url: &str) -> WasmReference {
        WasmReference {
            name: url.rsplit('/').next().unwrap_or(url).to_string(),
            url: url.to_string(),
        }
    }

    fn target(url: &str, base: Option<&str>) -> Option<String> {
        fetch_url(&reference(url), base).ok()
    }

    #[test]
    fn a_relative_url_is_fetched_only_against_an_absolute_base() {
        assert_eq!(target("pkg/a.wasm", None), None);
        assert_eq!(target("pkg/a.wasm", Some("pkg/")), None);
        assert_eq!(
            target("pkg/a.wasm", Some("https://x.dev/app/index.html")).as_deref(),
            Some("https://x.dev/app/pkg/a.wasm")
        );
        assert_eq!(
            target("/static/a.wasm", Some("https://x.dev/app/")).as_deref(),
            Some("https://x.dev/static/a.wasm")
        );
        assert_eq!(
            target("https://cdn.dev/a.wasm", None).as_deref(),
            Some("https://cdn.dev/a.wasm")
        );
    }

    #[test]
    fn a_base_that_is_only_an_origin_keeps_its_host() {
        assert_eq!(
            target("pkg/a.wasm", Some("https://x.dev")).as_deref(),
            Some("https://x.dev/pkg/a.wasm")
        );
    }

    #[test]
    fn a_url_that_cannot_be_requested_says_why() {
        let relative = fetch_url(&reference("pkg/a.wasm"), None).unwrap_err();
        let scheme = fetch_url(&reference("ftp://x.dev/a.wasm"), None).unwrap_err();
        let base = fetch_url(&reference("a.wasm"), Some("x.dev")).unwrap_err();
        assert!(
            relative.contains("pass --wasm-base to fetch it"),
            "{relative}"
        );
        assert!(scheme.contains("is not an http(s) URL"), "{scheme}");
        assert!(base.contains("against --wasm-base x.dev"), "{base}");
    }

    #[test]
    fn control_and_direction_characters_are_shown_escaped() {
        assert_eq!(printable("a\u{1b}[31mb"), "a\\u{1b}[31mb");
        assert_eq!(printable("a\r\nb\u{7f}"), "a\\u{d}\\u{a}b\\u{7f}");
        assert_eq!(printable("a\u{9b}b\u{2028}c"), "a\\u{9b}b\\u{2028}c");
        assert_eq!(printable("a\u{202e}b"), "a\\u{202e}b");
        assert_eq!(printable("a\u{200f}b\u{61c}c"), "a\\u{200f}b\\u{61c}c");
        assert_eq!(
            printable("a\u{200b}b\u{2060}c\u{feff}"),
            "a\\u{200b}b\\u{2060}c\\u{feff}"
        );
        assert_eq!(printable("plain é text"), "plain é text");
    }
}
