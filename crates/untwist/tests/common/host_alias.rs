#![allow(dead_code)]

use untwist::{Options, Pipeline};
use untwist_deobfuscate::proof::BUILTIN_ALIAS_BITS;

pub const PRELUDE: &str = include_str!("../data/host_alias/prelude.js");
const HEAD: &str = include_str!("../data/host_alias/head.js");
const PLAIN_HEAD: &str = include_str!("../data/host_alias/plain_head.js");
const BARE_HEAD: &str = include_str!("../data/host_alias/bare_head.js");
const ROWS: &str = include_str!("../data/host_alias/rows.txt");
const KEYED_ROWS: &str = include_str!("../data/host_alias/keyed_rows.txt");
const BARE_ROWS: &str = include_str!("../data/host_alias/bare_rows.txt");
const REVIEW_ROWS: &str = include_str!("../data/host_alias/review_rows.txt");
const STRING_HELD: &str = "var t=this;var Z=t[k];";
const GLOBAL_ROUTES: &str = include_str!("../data/host_alias/global_routes.txt");
const FOREIGN_REACH_ROWS: &str = include_str!("../data/host_alias/foreign_reach_rows.txt");
const GLOBAL_HELD: &str = "var t=this;var Z=t[k];";
const TAIL: &str = "console.log(dec0(0x1),dec0(0x2));";

fn framed(head: &str, rows: &str) -> Vec<(String, String)> {
    rows.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, patch)| {
            let head = head.trim_end();
            (
                label.to_string(),
                format!("(function(){{{head}{patch}{TAIL}}})();"),
            )
        })
        .collect()
}

pub fn rows() -> Vec<(String, String)> {
    let mut rows = framed(HEAD, ROWS);
    rows.extend(framed(PLAIN_HEAD, KEYED_ROWS));
    rows.extend(framed(BARE_HEAD, BARE_ROWS));
    rows.extend(framed(
        &format!("{}{STRING_HELD}", PLAIN_HEAD.trim_end()),
        REVIEW_ROWS,
    ));
    rows
}

pub fn foreign_reach_rows() -> Vec<(String, String)> {
    framed(
        &format!("{}{GLOBAL_HELD}", PLAIN_HEAD.trim_end()),
        FOREIGN_REACH_ROWS,
    )
}

pub fn plain_script(patch: &str) -> String {
    format!("{}{patch}", PLAIN_HEAD.trim_end())
}

pub fn global_routes() -> Vec<(String, String)> {
    framed(HEAD, GLOBAL_ROUTES)
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
