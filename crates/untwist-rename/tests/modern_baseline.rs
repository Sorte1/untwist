use untwist_rename::modern::baseline::{Baseline, Version};
use untwist_rename::modern::compat::{builtin, Verdict};
use untwist_rename::modern::parse_baseline;

fn engines(baseline: &Baseline) -> Vec<String> {
    baseline
        .engines()
        .iter()
        .map(|(engine, version)| format!("{engine} {version}"))
        .collect()
}

#[test]
fn an_es_year_stands_for_the_engines_in_the_generated_table() {
    let baseline = parse_baseline("es2022").unwrap();
    assert_eq!(
        engines(&baseline),
        [
            "chrome 94",
            "edge 94",
            "firefox 93",
            "safari 16.4",
            "ios 16.4",
            "node 16.11.0",
            "opera 80",
            "samsung 17.0"
        ]
    );
}

#[test]
fn engines_take_browserslist_aliases_and_an_engine_named_twice_keeps_its_lower_version() {
    let baseline = parse_baseline("es2022, Chrome 120, ff 121, ios_saf 17.2, chrome 100").unwrap();
    let listed = engines(&baseline);
    assert!(listed.contains(&"chrome 94".to_string()), "{listed:?}");
    assert!(listed.contains(&"firefox 93".to_string()), "{listed:?}");
    let only = parse_baseline("chrome 120, chrome 110").unwrap();
    assert_eq!(engines(&only), ["chrome 110"]);
}

#[test]
fn a_baseline_that_is_not_understood_is_refused_with_a_reason() {
    for (text, says) in [
        ("", "empty baseline"),
        ("es2099", "not a supported ES year"),
        ("netscape 4", "unknown engine"),
        ("chrome", "neither an ES year"),
        ("chrome x", "not an engine version"),
    ] {
        let error = parse_baseline(text).unwrap_err();
        assert!(error.contains(says), "{text:?}: {error}");
    }
}

#[test]
fn versions_compare_by_number_not_by_text() {
    let parse = |text: &str| Version::parse(text).unwrap();
    assert!(parse("16.11") > parse("16.9"));
    assert!(parse("10.0") == parse("10"));
    assert!(parse("122") > parse("94"));
    assert!(Version::parse("1.x").is_none());
}

#[test]
fn a_module_is_native_only_when_every_baseline_engine_has_it_in_every_matched_version() {
    let compat = builtin();
    let es2022 = parse_baseline("es2022").unwrap();
    let range = ("3.39.0", "3.39.0");
    assert_eq!(
        compat.verdict("es.string.ends-with", range, &es2022),
        Verdict::Native
    );
    assert_eq!(
        compat.verdict("es.object.has-own", range, &es2022),
        Verdict::Native
    );
    let Verdict::Needed { engine, needs, .. } = compat.verdict("es.array.push", range, &es2022)
    else {
        panic!("es.array.push is replaced below chrome 122");
    };
    assert_eq!((engine.as_str(), needs.as_deref()), ("chrome", Some("122")));
    assert!(matches!(
        compat.verdict("es.array.find-last", range, &es2022),
        Verdict::Needed { .. }
    ));
    assert_eq!(
        compat.verdict("esnext.no-such-module", range, &es2022),
        Verdict::Unlisted
    );
    let chrome = parse_baseline("chrome 125").unwrap();
    assert_eq!(
        compat.verdict("es.array.push", range, &chrome),
        Verdict::Native
    );
}

#[test]
fn an_engine_core_js_compat_does_not_list_needs_the_polyfill() {
    let compat = builtin();
    let rhino = parse_baseline("chrome 125, quest 1").unwrap();
    assert!(matches!(
        compat.verdict("es.string.ends-with", ("3.39.0", "3.39.0"), &rhino),
        Verdict::Needed { .. }
    ));
}

#[test]
fn a_range_takes_the_strictest_core_js_version_inside_it() {
    let compat = builtin();
    let chrome = parse_baseline("chrome 125").unwrap();
    let early = compat.verdict("es.array.push", ("3.0.1", "3.0.1"), &chrome);
    assert_eq!(early, Verdict::Unlisted);
    let wide = compat.verdict(
        "es.array.includes",
        ("3.39.0", "3.51.0"),
        &parse_baseline("firefox 101").unwrap(),
    );
    assert!(matches!(wide, Verdict::Needed { .. }), "{wide:?}");
}
