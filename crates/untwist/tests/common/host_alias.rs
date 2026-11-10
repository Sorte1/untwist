#![allow(dead_code)]

use untwist::{Options, Pipeline};
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;

pub const PRELUDE: &str = include_str!("../data/host_alias/prelude.js");
const HEAD: &str = include_str!("../data/host_alias/head.js");
const ROWS: &str = include_str!("../data/host_alias/rows.txt");
const GLOBAL_ROUTES: &str = include_str!("../data/host_alias/global_routes.txt");
const TAIL: &str = "console.log(dec0(0x1),dec0(0x2));";

fn framed(rows: &str) -> Vec<(String, String)> {
    rows.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, patch)| {
            let head = HEAD.trim_end();
            (
                label.to_string(),
                format!("(function(){{{head}{patch}{TAIL}}})();"),
            )
        })
        .collect()
}

pub fn rows() -> Vec<(String, String)> {
    framed(ROWS)
}

pub fn global_routes() -> Vec<(String, String)> {
    framed(GLOBAL_ROUTES)
}

pub fn hosted(rows: &[(String, String)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, program)| (label.clone(), format!("{PRELUDE}{program}")))
        .collect()
}

pub fn transformed(points_to: bool, source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        disabled_passes: if points_to {
            vec![BUILTIN_ALIAS_BITS]
        } else {
            Vec::new()
        },
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

pub fn transformed_under_host(points_to: bool, hosted: &str) -> String {
    let program = hosted.strip_prefix(PRELUDE).unwrap_or(hosted);
    format!("{PRELUDE}{}", transformed(points_to, program))
}

pub fn decoded(code: &str) -> bool {
    !code.contains("arr0")
}
