use untwist::{Options, Pipeline};
use untwist_testing::depth::{limits_for, scan, scan_as, Goal, C_P1};

const A_GIB_OF_ROOM: usize = 1 << 30;

fn deobfuscated(source: &str) -> String {
    let output = Pipeline::new(Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source);
    output.codes.concat()
}

fn nested_array(depth: usize) -> String {
    format!("{}1{}", "[".repeat(depth), "]".repeat(depth))
}

fn parse_site(json: &str) -> String {
    format!("var c = JSON.parse('{json}');\nconsole.log(c);\n")
}

fn nesting(text: &str, goal: Option<Goal>) -> u32 {
    let limits = limits_for(A_GIB_OF_ROOM, text.len());
    let found = match goal {
        Some(goal) => scan_as(text, &limits, goal),
        None => scan(text, &limits),
    };
    found
        .unwrap_or_else(|refusal| panic!("{refusal:?}"))
        .nesting
}

fn deepest_array_within(slack: u32) -> usize {
    (1..)
        .take_while(|depth| nesting(&nested_array(*depth), Some(Goal::Module)) <= slack)
        .last()
        .unwrap_or_else(|| panic!("no array fits a slack of {slack}"))
}

#[test]
fn a_small_json_text_still_inlines() {
    let output = deobfuscated(&parse_site("{\"a\":1,\"b\":[1,2,{\"c\":\"d\"}]}"));
    assert!(!output.contains("JSON.parse"), "{output}");
}

#[test]
fn a_json_text_inside_the_inputs_nesting_slack_inlines_and_one_level_more_is_kept() {
    let input_nesting = nesting(&parse_site(&nested_array(1)), None);
    let slack = input_nesting + C_P1;
    let deepest = deepest_array_within(slack);
    let inside = parse_site(&nested_array(deepest));
    assert_eq!(nesting(&inside, None), input_nesting);
    let output = deobfuscated(&inside);
    assert!(!output.contains("JSON.parse"), "{output}");
    let beyond = parse_site(&nested_array(deepest + 1));
    assert_eq!(nesting(&beyond, None), input_nesting);
    let output = deobfuscated(&beyond);
    assert!(output.contains("JSON.parse"), "{output}");
}

#[test]
fn a_json_text_far_beyond_the_slack_keeps_its_string() {
    let chained = format!("{}1{}", "{\"a\":1,\"b\":".repeat(300), "}".repeat(300));
    let output = deobfuscated(&parse_site(&chained));
    assert!(output.contains("JSON.parse"), "{}", output.len());
}
