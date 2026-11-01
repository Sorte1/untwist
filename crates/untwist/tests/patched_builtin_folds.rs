use untwist::{Options, Pipeline};
use untwist_testing::node::printed_by_node;

macro_rules! unit {
    ($patch:literal, $tail:literal) => {
        concat!(
            $patch,
            "\nfunction a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }\n",
            "function b(i){ var e=a(); return e[i] + '|' + (",
            $tail,
            "); }\nfunction later(){ return b(1); }\nconsole.log(b(0)); console.log(later()); console.log(b(2));"
        )
    };
}

struct Row {
    source: &'static str,
    node: &'static str,
    default: &'static str,
    conservative: &'static str,
}

fn modes(row: &Row) -> [(Options, &'static str); 3] {
    [
        (Options::default(), row.default),
        (
            Options {
                extended: false,
                ..Options::default()
            },
            row.conservative,
        ),
        (
            Options {
                extended: false,
                unsafe_transforms: true,
                ..Options::default()
            },
            row.conservative,
        ),
    ]
}

fn check(row: &Row) {
    if let Some(printed) = printed_by_node(row.source) {
        assert_eq!(printed, row.node, "node on the input");
    }
    for (options, prints) in modes(row) {
        let code = Pipeline::new(options).run(row.source).code().to_string();
        assert!(
            code.contains("console.log(b(0))"),
            "the decoder must refuse the unit: {code}"
        );
        if let Some(printed) = printed_by_node(&code) {
            assert_eq!(printed, prints, "node on the output: {code}");
        }
    }
}

const A_REPLACED_CHAR_AT: &str = unit!(
    "String.prototype.charAt = function(){ return 'X'; };",
    "'abc'.charAt(0)"
);

#[test]
fn a_replaced_char_at_is_not_folded() {
    check(&Row {
        source: A_REPLACED_CHAR_AT,
        node: "s0|X\ns1|X\ns2|X\n",
        default: "s0|X\ns1|X\ns2|X\n",
        conservative: "s0|X\ns1|X\ns2|X\n",
    });
}

const A_REPLACED_INDEX_OF: &str = unit!(
    "String.prototype.indexOf = function(){ return 7; };",
    "'abc'.indexOf('c')"
);

#[test]
fn a_replaced_index_of_is_not_folded() {
    check(&Row {
        source: A_REPLACED_INDEX_OF,
        node: "s0|7\ns1|7\ns2|7\n",
        default: "s0|7\ns1|7\ns2|7\n",
        conservative: "s0|7\ns1|7\ns2|7\n",
    });
}

const A_REPLACED_DECODE_URI_COMPONENT: &str = unit!(
    "decodeURIComponent = function(){ return 'X'; };",
    "decodeURIComponent('%41')"
);

#[test]
fn a_replaced_decode_uri_component_is_still_evaluated_in_every_mode_as_reference_does() {
    check(&Row {
        source: A_REPLACED_DECODE_URI_COMPONENT,
        node: "s0|X\ns1|X\ns2|X\n",
        default: "s0|A\ns1|A\ns2|A\n",
        conservative: "s0|A\ns1|A\ns2|A\n",
    });
}

const A_DYNAMIC_KEY_WRITE_ON_THE_STRING_PROTOTYPE: &str = unit!(
    "function key(){ return 'charAt'; }\nString.prototype[key()] = function(){ return 'X'; };",
    "'abc'.charAt(0)"
);

#[test]
fn a_dynamic_key_write_on_the_string_prototype_stops_the_fold() {
    check(&Row {
        source: A_DYNAMIC_KEY_WRITE_ON_THE_STRING_PROTOTYPE,
        node: "s0|X\ns1|X\ns2|X\n",
        default: "s0|X\ns1|X\ns2|X\n",
        conservative: "s0|X\ns1|X\ns2|X\n",
    });
}
