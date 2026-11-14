use untwist_unpack::webpack::library_head::recorded_head_digests;

const RECORDED: &str = include_str!("data/webpack_library_heads.txt");
const FORMS: &str = include_str!("../src/webpack/library_head_forms.rs");

#[test]
fn the_recorded_library_heads_match_their_digests() {
    let digests = recorded_head_digests(RECORDED);
    let listed: Vec<String> = FORMS
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    let expected = format!(
        "pub const HEAD_FORMS: &[&str] = &[\n{}];\n",
        digests
            .iter()
            .map(|digest| format!("    \"{digest}\",\n"))
            .collect::<String>()
    );
    assert_eq!(
        listed, digests,
        "replace src/webpack/library_head_forms.rs with:\n{expected}"
    );
}
