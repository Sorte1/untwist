use untwist::{Options, Pipeline, OWN_FILE_READ};

const SELF_HASH: &str = r#"var _req = require;
function seal() {
  var text = _req("node:fs").readFileSync(__filename);
  var digest = _req("node:crypto").createHash("sha256").update(text).digest("hex");
  if (digest !== "0000000000000000000000000000000000000000000000000000000000000000") {
    throw new Error("integrity check failed");
  }
}
var greeting = "hel" + "lo";
if (!0) console.log(greeting);
"#;

const NO_SELF_READ: &str = r#"var path = require("node:path");
var greeting = "hel" + "lo";
if (!0) console.log(greeting, path.basename(__filename));
"#;

fn lanes(source: &str) -> Vec<(String, bool)> {
    [false, true]
        .into_iter()
        .map(|unsafe_transforms| {
            let output = Pipeline::new(Options {
                unsafe_transforms,
                commonjs_goal: true,
                ..Options::default()
            })
            .run(source);
            let refused = output
                .notes
                .iter()
                .any(|note| note.message == OWN_FILE_READ);
            (output.code().to_string(), refused)
        })
        .collect()
}

#[test]
fn an_input_that_hashes_its_own_file_is_passed_through_in_both_lanes() {
    for (code, refused) in lanes(SELF_HASH) {
        assert!(refused, "no own-file note");
        assert_eq!(code, SELF_HASH);
    }
}

#[test]
fn an_input_that_only_names_its_own_file_is_still_processed() {
    for (code, refused) in lanes(NO_SELF_READ) {
        assert!(!refused);
        assert!(code.contains("\"hello\""), "{code}");
    }
}
