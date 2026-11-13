use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ADD: &str = "({abcde:function(a,b){return a+b}})";
const SUB: &str = "({fghij:function(a,b){return a-b}})";

fn nest(object: &str, key: &str, depth: usize, leaf: &str, step: &str) -> String {
    format!(
        "{}{leaf}{}",
        format!("{object}.{key}(").repeat(depth),
        format!(",{step})").repeat(depth)
    )
}

fn tracked(depth: usize) -> String {
    let mut text = "t(0)".to_string();
    for index in 1..=depth {
        text = format!("{SUB}.fghij({text},t({index}))");
    }
    format!("var s=[];function t(i){{s.push(i);return i}}console.log({text},s.join());")
}

fn rows() -> Vec<(String, String, bool)> {
    let mut rows = Vec::new();
    for depth in [1, 2, 3, 5, 40, 120, 200, 256] {
        rows.push((
            format!("add-{depth}"),
            format!("console.log({});", nest(ADD, "abcde", depth, "1", "1")),
            true,
        ));
        rows.push((
            format!("strings-{depth}"),
            format!("console.log({});", nest(ADD, "abcde", depth, "'a'", "'b'")),
            true,
        ));
        rows.push((format!("order-{depth}"), tracked(depth), true));
    }
    rows.push((
        "past-the-limit".to_string(),
        format!("console.log({});", nest(ADD, "abcde", 300, "1", "1")),
        false,
    ));
    rows.push((
        "siblings".to_string(),
        format!(
            "console.log([{}, {}, {}]);",
            nest(ADD, "abcde", 3, "1", "2"),
            nest(SUB, "fghij", 60, "100", "1"),
            nest(ADD, "abcde", 300, "0", "1")
        ),
        false,
    ));
    rows.push((
        "named-member-inside".to_string(),
        format!(
            "console.log({});",
            nest(
                ADD,
                "abcde",
                30,
                "({klmno:function klmno(){return 4}}).klmno()",
                "({pqrst:'z'}).pqrst"
            )
        ),
        true,
    ));
    rows.push((
        "logical-inner".to_string(),
        format!(
            "var c=0;function g(){{c++;return c}}console.log({ADD}.abcde({}.uvwxy(g(),g()),{}),c);",
            "({uvwxy:function(a,b){return a||b}})",
            nest(ADD, "abcde", 20, "1", "1")
        ),
        false,
    ));
    rows
}

fn run(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_literal_control_flow_nest_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .map(|(label, program, _)| (label, program))
        .collect();
    let default = |source: &str| run(false, source);
    let unsafe_lane = |source: &str| run(true, source);
    let modes: [Mode<'_>; 2] = [("default", &default), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_nest_within_the_limit_is_inlined_whole_in_both_lanes() {
    let mut kept = Vec::new();
    for (label, program, whole) in rows() {
        if !whole {
            continue;
        }
        for (lane, unsafe_transforms) in [("default", false), ("unsafe", true)] {
            let code = run(unsafe_transforms, &program);
            if code.contains("abcde") || code.contains("fghij") {
                kept.push(format!("{label} ({lane}) kept a call:\n{code}"));
            }
        }
    }
    assert!(kept.is_empty(), "{}", kept.join("\n"));
}

#[test]
fn a_nest_past_the_limit_keeps_every_call() {
    let code = run(
        false,
        &format!("x = {};", nest(ADD, "abcde", 300, "1", "1")),
    );
    assert_eq!(code.matches(".abcde(").count(), 300, "{code}");
}
