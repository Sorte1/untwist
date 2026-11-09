use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const ENTRY: &str = "index.js";
const SOURCE: &str = "bundle.js";

pub struct Unpacked {
    dir: tempfile::TempDir,
    bundle: String,
}

impl Unpacked {
    pub fn of(bundle: &str, conservative: bool) -> Self {
        let options = Options {
            extended: !conservative,
            deobfuscate: false,
            unminify: false,
            ..Options::default()
        };
        let out = Pipeline::new(options).run(bundle);
        let dir = tempfile::tempdir().expect("temp dir");
        out.write_to(dir.path()).expect("write");
        Self {
            dir,
            bundle: bundle.to_string(),
        }
    }

    pub fn file(&self, path: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(path))
            .unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    pub fn entry(&self) -> String {
        self.file(ENTRY)
    }

    pub fn runs_like_the_bundle(&self, expected: &str) {
        let Some(node) = node() else {
            let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
            assert!(
                skip_allowed(required.as_deref()),
                "required: {SKIP_MESSAGE}"
            );
            return;
        };
        let source = self.dir.path().join(SOURCE);
        std::fs::write(&source, &self.bundle).expect("write the bundle");
        let bundle = run(&node, &source).expect("spawn").expect("terminates");
        let tree = run(&node, &self.dir.path().join(ENTRY))
            .expect("spawn")
            .expect("terminates");
        assert_eq!(bundle.stdout, expected, "{}", bundle.stderr);
        assert_eq!(tree.stdout, expected, "{}\n{}", tree.stderr, self.entry());
    }
}
