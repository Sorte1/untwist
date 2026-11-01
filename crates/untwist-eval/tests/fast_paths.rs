use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{record_units_charged, UnitRecord};
use untwist_testing::node::printed_by_node;

struct Row {
    source: &'static str,
    node: &'static str,
    steps: u64,
}

const fn row(source: &'static str, node: &'static str, steps: u64) -> Row {
    Row {
        source,
        node,
        steps,
    }
}

fn units(source: &str, fuel: Option<u64>, decoding: bool) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{source}");
    let budget = fuel.map_or_else(Budget::default, Budget::new);
    record_units_charged(|| match decoding {
        true => eval_script_decoding(&parsed.program, budget, |_| ()),
        false => eval_script_in(&parsed.program, budget, |_, _| ()),
    })
}

fn call(row: &Row) -> String {
    format!("String((function () {{ {} }})())", row.source)
}

fn answers_as_node_at_the_pinned_weight(rows: &[Row]) {
    let mut wrong = Vec::new();
    for row in rows {
        let source = format!("{};", call(row));
        if let Some(printed) = printed_by_node(&format!("console.log({});", call(row))) {
            assert_eq!(printed.trim_end(), row.node, "node on {}", row.source);
        }
        let outside = units(&source, None, false);
        assert_eq!(
            outside[0].values,
            [format!("string:{}", row.node)],
            "{}",
            row.source
        );
        if outside[0].steps_used != row.steps {
            wrong.push(format!(
                "{} steps, pinned {}: {}",
                outside[0].steps_used, row.steps, row.source
            ));
            continue;
        }
        let decoding = units(&source, None, true);
        if decoding[0].bail.is_none() {
            assert_eq!(
                decoding[0].values,
                [format!("string:{}", row.node)],
                "{}",
                row.source
            );
        }
        for cap in 0..=row.steps {
            let capped = units(&source, Some(cap), false);
            assert!(
                capped[0].bail.is_some() == (cap < row.steps),
                "{} at fuel {cap}: {:?}",
                row.source,
                capped[0]
            );
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

const OPERATORS: &[Row] = &[
    row(
        "var a = 7, b = -2, m = -4, t = 2, h = 5.5, z = 0; return [a % b, -a % b, m % t, 1 / (m % t), h % t, a % z, a % h, -h % b].join();",
        "1,-1,0,-Infinity,1.5,NaN,1.5,-1.5",
        76,
    ),
    row(
        "var big = 9007199254740993, huge = 1e300, n = NaN, i = Infinity, s = 3; return [big % s, huge % s, n % s, i % s, s % i, -0 % s, 1 / (-0 % s)].join();",
        "2,0,NaN,NaN,3,0,-Infinity",
        65,
    ),
    row(
        "var a = 7, b = -2, n = NaN; return [a < b, a <= n, n >= n, a > b, b >= b, a == 7, a != 7, a === 7, a !== b, n == n, n !== n].join();",
        "false,false,false,true,true,true,false,true,true,false,true",
        85,
    ),
    row(
        "var a = 7, b = -2, c = 33, e = -8, f = -1, g = 0, x = 4294967296.5; return [a - b, a * b, a / b, a + b, a & b, a | b, a ^ b, a << c, e >> 1, f >>> g, x | g, x >>> g].join();",
        "9,-14,-3.5,5,6,-1,-7,14,-4,4294967295,0,0",
        96,
    ),
    row(
        "var a = 7, s = '7', o = {v: 2}, k = 'v'; a += 1; a %= 3; a -= -0.5; s += 1; o.v *= a; o[k] **= 2; var i = 1.5, j = '2', u; i++; ++i; j--; u++; return [a, s, o.v, i, j, u].join();",
        "2.5,71,25,3.5,1,NaN",
        81,
    ),
];

#[test]
fn a_number_pair_takes_the_operator_fast_path_with_node_values_and_the_same_weight() {
    answers_as_node_at_the_pinned_weight(OPERATORS);
}

const ELEMENTS: &[Row] = &[
    row(
        "var a = [1, 'b', undefined, null], i = 1, z = -0, h = 1.5, n = NaN, m = -1, far = 4294967295; a[m] = 'neg'; a['x'] = 'name'; return [a[0], a[i], a[2], a[3], a[z], a[h], a[n], a[m], a[9], a[far], a['x'], a[i + 1] === undefined].join('|');",
        "1|b|||1|||neg|||name|true",
        111,
    ),
    row(
        "var s = 'h\\u00e9llo', i = 1, z = -0, h = 0.5, n = 5, m = -1; return [s[0], s[i], s[z], s[h], s[n], s[m], s[i] === s.charAt(1), s[4].length].join('|');",
        "h|\u{e9}|h||||true|1",
        76,
    ),
    row(
        "var a = [[1, 2], [3, 4]], o = null, i = 1, t = 0; try { o[i]; } catch (e) { t = 1; } return [a[i][0], a[0][i], a?.[i]?.[i], o?.[i], t, a?.[i][0]].join('|');",
        "3|2|4||1|3",
        76,
    ),
    row(
        "var r = (function () { return [arguments[0], arguments[1], arguments.length]; })(5, 6); var a = [7], k = 0; a[k] = 8; a[1] = 9; return [r.join(), a[k], a[1], a.length].join('|');",
        "5,6,2|8|9|2",
        73,
    ),
];

#[test]
fn an_integer_key_on_an_array_or_string_takes_the_element_path_with_node_values_and_the_same_weight(
) {
    answers_as_node_at_the_pinned_weight(ELEMENTS);
}
