use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const GZIP_HEADER: usize = 10;
const GZIP_TRAILER: usize = 8;
const SUFFIX: &str = ".table.gz";

fn check(path: &Path) {
    let data = std::fs::read(path).expect("reading a library table");
    assert!(
        data.len() > GZIP_HEADER + GZIP_TRAILER && data[..3] == [0x1f, 0x8b, 8] && data[3] == 0,
        "{} is not a plain gzip file",
        path.display()
    );
}

fn tables(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            let id = name.strip_suffix(SUFFIX)?.to_string();
            Some((id, path))
        })
        .collect();
    found.sort();
    found
}

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let dir = Path::new(&manifest).join("data");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut code = String::from("pub static TABLES: &[(&str, &[u8])] = &[\n");
    for (id, path) in tables(&dir) {
        println!("cargo:rerun-if-changed={}", path.display());
        check(&path);
        let path = path.to_str().expect("the table path is UTF-8").to_string();
        writeln!(code, "    ({id:?}, include_bytes!({path:?})),").expect("writing to a String");
    }
    code.push_str("];\n");
    std::fs::write(out.join("library_tables.rs"), code).expect("writing the table list");
}
