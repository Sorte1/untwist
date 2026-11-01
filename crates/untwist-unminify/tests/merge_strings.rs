use untwist_testing::assert_transform;

#[test]
fn only_strings() {
    assert_transform!(
        r#""a" + "b" + "c";"#,
        [untwist_unminify::merge_strings::boxed],
        @r#"("abc");"#
    );
}

#[test]
fn with_variables() {
    assert_transform!(
        r#""a" + "b" + xyz + "c" + "d";"#,
        [untwist_unminify::merge_strings::boxed],
        @r#""ab" + xyz + "cd";"#
    );
}

#[test]
fn ignore_lone_surrogate_left() {
    assert_transform!(
        r#"var s = "\ud800" + "b";"#,
        [untwist_unminify::merge_strings::boxed],
        @r#"var s = "\ud800" + "b";"#
    );
}

#[test]
fn ignore_lone_surrogate_right() {
    assert_transform!(
        r#"var t = "a" + "\udfff";"#,
        [untwist_unminify::merge_strings::boxed],
        @r#"var t = "a" + "\udfff";"#
    );
}

#[test]
fn a_three_literal_chain_becomes_one_literal() {
    assert_transform!(
        "x = 'a' + 'b' + 'c';",
        [untwist_unminify::merge_strings::boxed],
        @r#"x = "abc";"#
    );
}

#[test]
fn a_chain_broken_by_an_identifier_merges_only_after_it() {
    assert_transform!(
        "x = 'a' + x + 'b' + 'c';",
        [untwist_unminify::merge_strings::boxed],
        @r#"x = "a" + x + "bc";"#
    );
}

#[test]
fn the_change_count_is_one_per_merged_link() {
    let alloc = untwist_testing::Allocator::default();
    let (_, report) = untwist_testing::transform_report(
        &alloc,
        "x = 'a' + 'b' + 'c' + 'd'; y = z + 'a' + 'b' + 'c';",
        &[untwist_unminify::merge_strings::boxed],
        untwist_core::pass::PassOptions::default(),
    );
    assert_eq!(report.per_pass, [("merge-strings", 5)]);
}

#[cfg(target_os = "linux")]
mod arena {
    use untwist_core::parse::parse_program;
    use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
    use untwist_core::print::{print, PrintStyle};
    use untwist_core::semantic::build_scoping;
    use untwist_testing::capped_child::{self, Cap};
    use untwist_testing::depth_classes::{generate, Class};
    use untwist_testing::Allocator;

    const CHILD: &str = "UNTWIST_TEST_M3_CHILD";
    const LINKS: usize = 10_000;
    const FIXED_OVERHEAD: usize = 64 << 10;
    const ROW: &str = "arena::arena_bytes_after_a_ten_thousand_link_chain_are_linear";

    fn pass_arena_bytes(source: &str) -> (usize, String) {
        let alloc = Allocator::default();
        let mut parsed = parse_program(&alloc, source);
        let sem = build_scoping(&parsed.program);
        let before = alloc.used_bytes();
        let mut group = PassGroup::from_ctors(
            &[untwist_unminify::merge_strings::boxed],
            PassOptions::default(),
        );
        let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
        let used = alloc.used_bytes() - before;
        (used, print(&parsed.program, PrintStyle::Compact))
    }

    fn arena_row() {
        let source = generate(Class::Strplus, LINKS);
        let worker = std::thread::Builder::new()
            .stack_size(512 << 20)
            .spawn(move || pass_arena_bytes(&source))
            .expect("a thread for the deep chain");
        let (used, printed) = worker.join().expect("the chain row ran");
        let result_len = LINKS + 1;
        assert!(
            !printed.contains('+'),
            "the chain was not merged into one literal"
        );
        assert_eq!(printed.matches('a').count(), result_len);
        assert!(
            used <= 2 * result_len + FIXED_OVERHEAD,
            "the pass took {used} arena bytes for a {result_len}-byte result"
        );
    }

    #[test]
    fn arena_bytes_after_a_ten_thousand_link_chain_are_linear() {
        if std::env::var_os(CHILD).is_some() {
            arena_row();
            return;
        }
        let exe = std::env::current_exe().expect("the test binary");
        let args = [ROW, "--exact", "--nocapture", "--test-threads=1"].map(Into::into);
        let env = [(CHILD.into(), "1".into())];
        let outcome = capped_child::run(exe.as_os_str(), &args, &env, None, Cap::DEFAULT);
        assert_eq!(
            outcome.exit_code,
            Some(0),
            "{}{}",
            String::from_utf8_lossy(&outcome.stdout),
            String::from_utf8_lossy(&outcome.stderr)
        );
    }
}
