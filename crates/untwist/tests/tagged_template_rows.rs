use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: [(&str, &str); 8] = [
    (
        "tg09 a string substitution is an argument of Function, not body text",
        r#"Function`a${'String = Array'}`(); String.prototype.join = function () { return "X"; }; console.log([1, 2].join('+'));"#,
    ),
    (
        "tg08 a substitution leaves Function a parameter list it cannot parse",
        r#"try { Function`${'String = Array'}`(); } catch (e) { console.log(e.name); } console.log([1, 2].join('+'));"#,
    ),
    (
        "a tag sees its strings, raw strings and substitutions",
        r#"function tag(s, ...v) { return JSON.stringify([s, s.raw, v]); } console.log(tag`a${'b'}c${'d'}`);"#,
    ),
    (
        "String.raw keeps an escape that a substitution holds",
        r#"console.log(String.raw`a${'\n'}b`.length);"#,
    ),
    (
        "a tag counts its substitutions",
        r#"function count(s, ...v) { return s.length + ':' + v.length; } console.log(count`x${'y'}z`);"#,
    ),
    (
        "a template inside a substitution of a tag",
        r#"function tag(s, ...v) { return s.join('|') + '/' + v.join('|'); } console.log(tag`x${`y${'z'}`}w`);"#,
    ),
    (
        "a member tag",
        r#"var o = { t: function (s, v) { return s.length + v; } }; console.log(o.t`a${'b'}c`);"#,
    ),
    (
        "an untagged template still takes its string substitutions",
        r#"console.log(`a${'b'}c`, `${'x'}${1}`);"#,
    ),
];

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

#[test]
fn a_tagged_template_keeps_its_string_substitutions() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let conservative = Options {
        extended: false,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let run_conservative = |source: &str| transformed(&conservative, source);
    let modes: [Mode<'_>; 3] = [
        ("default", &run_safe),
        ("unsafe", &run_unsafe),
        ("conservative", &run_conservative),
    ];
    let programs: Vec<(String, String)> = ROWS
        .iter()
        .map(|(label, source)| (label.to_string(), format!("{source}\n")))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
