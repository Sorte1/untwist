use untwist_rename::human::looks_human_written;

#[test]
fn obfuscated_names_are_not_human_written() {
    let generated = [
        "a",
        "b",
        "z",
        "_",
        "$",
        "_0x3bd1",
        "_0xabc",
        "__0x1f",
        "_0X4C3E",
        "IÏïïïìi",
        "llIlIl",
        "ıíīįl",
        "Il",
        "Wq",
        "bTx",
        "xQfP",
        "Zt",
        "vLN1",
    ];
    for name in generated {
        assert!(!looks_human_written(name), "{name} must read as generated");
    }
}

#[test]
fn real_names_are_human_written() {
    let written = [
        "handler",
        "getUser",
        "user",
        "fn",
        "req",
        "res",
        "err",
        "videoRef",
        "setExportProgress",
        "lib_someLibA",
        "_private",
        "$el",
        "café",
        "日本語",
        "naïve",
        "Ünicode",
        "fn1",
        "arr2",
        "v17",
    ];
    for name in written {
        assert!(looks_human_written(name), "{name} must read as written");
    }
}

#[test]
fn the_empty_name_is_not_human_written() {
    assert!(!looks_human_written(""));
}
