use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use untwist::{Output, SuppliedWasm, WasmOrigin, WasmReference, WasmSupply};

use crate::args::Args;

const MAX_WASM_BYTES: u64 = untwist::MAX_WASM_BYTES as u64;
const FETCH_TIMEOUT_SECS: u64 = 30;

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
                Err(error) => writeln!(err, "warning: read {}: {error}", path.display())?,
            }
        }
    }
    Ok(WasmSupply { files })
}

fn is_absolute(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

fn joined(base: &str, url: &str) -> String {
    match url.strip_prefix('/') {
        Some(rooted) => {
            let origin_end = base
                .find("://")
                .and_then(|scheme| base[scheme + 3..].find('/').map(|at| scheme + 3 + at))
                .unwrap_or(base.len());
            format!("{}/{rooted}", &base[..origin_end])
        }
        None => {
            let directory = match base.ends_with('/') {
                true => base,
                false => base.rsplit_once('/').map_or(base, |(head, _)| head),
            };
            format!(
                "{}/{}",
                directory.trim_end_matches('/'),
                url.trim_start_matches("./")
            )
        }
    }
}

pub fn fetch_url(reference: &WasmReference, base: Option<&str>) -> Option<String> {
    if is_absolute(&reference.url) {
        return Some(reference.url.clone());
    }
    let base = base.filter(|base| is_absolute(base))?;
    Some(joined(base, &reference.url))
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(FETCH_TIMEOUT_SECS)))
        .build()
        .into();
    let response = agent.get(url).call().map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_WASM_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
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
    for reference in result.wasm_references() {
        if supply.find(&reference.name).is_some() {
            continue;
        }
        let Some(url) = fetch_url(&reference, args.wasm_base.as_deref()) else {
            writeln!(
                err,
                "note: not fetching {}: {} is relative; pass --wasm-base to fetch it",
                reference.name, reference.url
            )?;
            continue;
        };
        match fetch(&url) {
            Ok(bytes) => supply.files.push(SuppliedWasm {
                name: reference.name.clone(),
                bytes,
                origin: WasmOrigin::Fetched,
                url: Some(url),
            }),
            Err(error) => writeln!(err, "warning: fetching {url} failed: {error}")?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(url: &str) -> WasmReference {
        WasmReference {
            name: untwist_unpack_name(url),
            url: url.to_string(),
        }
    }

    fn untwist_unpack_name(url: &str) -> String {
        url.rsplit('/').next().unwrap_or(url).to_string()
    }

    #[test]
    fn a_relative_url_is_fetched_only_against_an_absolute_base() {
        assert_eq!(fetch_url(&reference("pkg/a.wasm"), None), None);
        assert_eq!(fetch_url(&reference("pkg/a.wasm"), Some("pkg/")), None);
        assert_eq!(
            fetch_url(
                &reference("pkg/a.wasm"),
                Some("https://x.dev/app/index.html")
            )
            .as_deref(),
            Some("https://x.dev/app/pkg/a.wasm")
        );
        assert_eq!(
            fetch_url(&reference("/static/a.wasm"), Some("https://x.dev/app/")).as_deref(),
            Some("https://x.dev/static/a.wasm")
        );
        assert_eq!(
            fetch_url(&reference("https://cdn.dev/a.wasm"), None).as_deref(),
            Some("https://cdn.dev/a.wasm")
        );
    }
}
