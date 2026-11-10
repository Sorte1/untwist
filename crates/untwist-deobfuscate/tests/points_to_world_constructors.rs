use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;
use untwist_deobfuscate::mutants::drop_world_constructors_for_tests;
use untwist_deobfuscate::proof::points_to_sites;

const STORES_THROUGH_A_FOREIGN_CONSTRUCTOR: &[&str] = &[
    "var c = host.text().constructor; c.fromCharCode = 1;",
    "var p = host.text().__proto__; p.slice = 1;",
    "host.listen(function (e) { var c = e.constructor; c.prototype.charAt = 1; });",
    "var c = host.text().constructor; c.prototype[host.key()] = 1;",
];

fn site_count(source: &str) -> usize {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, SourceType::cjs()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    points_to_sites(&parsed.program, &scoping, &FxHashMap::default(), &[]).len()
}

#[test]
fn constructors_read_off_foreign_values_are_patch_sites_only_while_the_rule_holds() {
    let kept: Vec<usize> = STORES_THROUGH_A_FOREIGN_CONSTRUCTOR
        .iter()
        .map(|source| site_count(source))
        .collect();
    drop_world_constructors_for_tests(true);
    let dropped: Vec<usize> = STORES_THROUGH_A_FOREIGN_CONSTRUCTOR
        .iter()
        .map(|source| site_count(source))
        .collect();
    drop_world_constructors_for_tests(false);
    for ((source, kept), dropped) in STORES_THROUGH_A_FOREIGN_CONSTRUCTOR
        .iter()
        .zip(kept)
        .zip(dropped)
    {
        assert!(kept > 0, "{source}: no site");
        assert_eq!(dropped, 0, "{source}: a site without the rule");
    }
}
