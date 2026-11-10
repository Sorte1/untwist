use untwist::{Options, Pipeline};

const TEXT_UNITS: usize = 1 << 18;

fn long_text_sites(sites: usize) -> String {
    let long = "x".repeat(TEXT_UNITS);
    let calls = "console.log(b(0x0));".repeat(sites);
    format!(
        "function a() {{ var h = ['{long}', 'log']; a = function () {{ return h; }}; return a(); }}\n\
         function b(c, d) {{ var e = a(); b = function (f, g) {{ f = f - 0x0; return e[f]; }}; return b(c, d); }}\n\
         {calls}\n"
    )
}

fn peak(sites: usize) -> u64 {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    let out = Pipeline::new(options).run(&long_text_sites(sites));
    assert!(
        out.code().contains(&"x".repeat(64)),
        "the long text was not decoded"
    );
    out.memory_peak
}

#[test]
fn one_long_decoded_text_read_at_many_sites_is_held_once_by_the_alias_check() {
    let few = peak(50);
    let many = peak(400);
    let per_site = many.saturating_sub(few) / 350;
    assert!(
        per_site < TEXT_UNITS as u64,
        "each extra call site of a {TEXT_UNITS}-unit text costs {per_site} bytes ({few} -> {many})"
    );
}
