use untwist::{Options, Pipeline};

#[test]
fn readme_library_example_runs_and_writes_output() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let out_dir = dir.path().join("out-dir");

    let output = Pipeline::new(Options::default()).run("var a = !0;");
    output.write_to(&out_dir)?;

    assert_eq!(output.code(), "var a = true;");
    assert!(out_dir.join(untwist::DEOBFUSCATED).is_file());
    Ok(())
}
