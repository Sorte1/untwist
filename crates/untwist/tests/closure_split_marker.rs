use untwist::{Options, Pipeline};

const BUNDLE: &str = r#"this._s = this._s || {};
(function (_) {
  var window = this;
  try {
    _._ModuleManager_initialize = function () {};
    _.lib = {};
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:a*/
    var first = 1, second = 2;
    _.lib.a = first + second;
  } catch (e) {
    _._DumpException(e);
  }
  try {
    /*_M:b*/
    console.log("eval b", _.lib.a);
  } catch (e) {
    _._DumpException(e);
  }
}).call(this, this._s);
"#;

fn split(options: Options) -> Option<(String, usize)> {
    Pipeline::new(options)
        .run(BUNDLE)
        .bundle
        .map(|bundle| (bundle.kind, bundle.modules.len()))
}

#[test]
fn a_segment_opening_with_a_multi_declarator_var_keeps_its_marker() {
    let unpack_only = Options {
        deobfuscate: false,
        unminify: false,
        jsx: false,
        ..Options::default()
    };
    let want = Some(("closure".to_string(), 4));
    assert_eq!(split(unpack_only), want, "unpack-only");
    assert_eq!(split(Options::default()), want, "default");
}
