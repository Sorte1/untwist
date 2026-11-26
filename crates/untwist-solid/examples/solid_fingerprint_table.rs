use std::collections::BTreeMap;

use untwist_solid::FingerprintRow;

fn rows(path: &str) -> Vec<FingerprintRow> {
    let code = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"));
    untwist_solid::fingerprint_rows(&code)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if std::env::var_os("DUMP").is_some() {
        for path in &args {
            for row in rows(path) {
                println!("{path}\t{}\t{:016x}\t{}", row.name, row.hash, row.nodes);
            }
        }
        return;
    }
    let kept = untwist_restore::table::table(&args, &rows);
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, rt) in &kept {
        *count.entry(rt.as_str()).or_default() += 1;
    }
    eprintln!("{} rows: {count:?}", kept.len());
    println!("use crate::runtime::{{bundled_name, Rt}};\n");
    println!("const TABLE: &[(u64, &str)] = &[");
    for (hash, rt) in &kept {
        println!("    (0x{hash:016x}, \"{rt}\"),");
    }
    println!("];");
    println!();
    println!("pub fn lookup(hash: u64) -> Option<Rt> {{");
    println!("    TABLE");
    println!("        .binary_search_by_key(&hash, |row| row.0)");
    println!("        .ok()");
    println!("        .and_then(|at| bundled_name(TABLE[at].1))");
    println!("}}");
}
