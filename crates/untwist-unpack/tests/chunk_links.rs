use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack_inputs, BundleKind, UnpackOptions};

const BUNDLERS: &str = "../../fixtures/unpack-bundlers";
const HOIST: [&str; 2] = ["parcel/hoist/index.js", "parcel/hoist/lazy.58e54a87.js"];
const NOHOIST: [&str; 2] = ["parcel/nohoist/index.js", "parcel/nohoist/lazy.99d34c29.js"];
const SERVER: [&str; 4] = [
    "turbopack/min-server/index.js",
    "turbopack/min-server/server/chunks/ssr/[turbopack]_runtime.js",
    "turbopack/min-server/server/chunks/ssr/app.js",
    "turbopack/min-server/server/chunks/ssr/lib.js",
];
const HOIST_LOADER: &str = "parcelRegister(\"kFPqQ\", function(module, exports) {\n";
const HOIST_DATA: &str = "module.exports = JSON.parse(";
const REGISTRY_LOADER: &str = "module.exports = Promise.resolve(module.bundle.root(\"hmtXX\"));";

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(BUNDLERS)
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

fn unpacked(sources: &[String]) -> Option<(BundleKind, Vec<(String, String)>)> {
    let alloc = Allocator::default();
    let mut programs: Vec<Program<'_>> = sources
        .iter()
        .map(|source| {
            let parsed = parse_program(&alloc, source);
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            parsed.program
        })
        .collect();
    let (bundle, _) = unpack_inputs(&alloc, &mut programs, &[], UnpackOptions::default());
    let bundle = bundle?;
    let files = bundle
        .modules
        .values()
        .map(|module| {
            (
                module.path.as_str().to_string(),
                pretty_codegen(module.program()),
            )
        })
        .collect();
    Some((bundle.kind, files))
}

fn kind(sources: &[String]) -> Option<BundleKind> {
    unpacked(sources).map(|(kind, _)| kind)
}

fn edited(names: [&str; 2], edit: impl Fn(String) -> String) -> Vec<String> {
    let [root, lazy] = names.map(fixture);
    vec![edit(root), lazy]
}

fn replaced(source: String, anchor: &str, with: &str) -> String {
    assert!(source.contains(anchor), "anchor {anchor:?} missing");
    source.replacen(anchor, with, 1)
}

#[test]
fn the_parcel_fixtures_with_their_lazy_chunks_still_link() {
    assert_eq!(kind(&HOIST.map(fixture)), Some(BundleKind::Parcel));
    assert_eq!(kind(&NOHOIST.map(fixture)), Some(BundleKind::Parcel));
}

#[test]
fn a_hoisted_entry_that_requires_a_lazy_module_before_its_chunk_loads_is_not_linked() {
    let sources = edited(HOIST, |root| {
        let at = root
            .rfind("var $66COJ = parcelRequire(\"66COJ\");")
            .expect("entry");
        let mut root = root;
        root.insert_str(at, "console.log(\"early\", parcelRequire(\"ibiaQ\"));\n");
        root
    });
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_hoisted_module_that_requires_a_lazy_module_outside_the_loader_is_not_linked() {
    let sources = edited(HOIST, |root| {
        replaced(
            root,
            HOIST_DATA,
            "parcelRequire(\"ibiaQ\");\nmodule.exports = JSON.parse(",
        )
    });
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_hoisted_loader_with_a_second_lazy_require_is_not_linked() {
    let sources = edited(HOIST, |root| {
        replaced(
            root,
            HOIST_LOADER,
            "parcelRegister(\"kFPqQ\", function(module, exports) {\nparcelRequire(\"ibiaQ\");\n",
        )
    });
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_registry_module_that_reaches_a_lazy_module_through_the_root_is_not_linked() {
    let sources = edited(NOHOIST, |root| {
        replaced(
            root,
            REGISTRY_LOADER,
            "module.exports = Promise.resolve(module.bundle.root(\"giiHr\"));",
        )
    });
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_registry_module_that_requires_a_lazy_module_by_its_map_is_not_linked() {
    let sources = edited(NOHOIST, |root| {
        let root = replaced(
            root,
            REGISTRY_LOADER,
            "module.exports = Promise.resolve(require(\"./lazy\"));",
        );
        replaced(
            root,
            "},{}],\"jjwCr\"",
            "},{\"./lazy\":\"giiHr\"}],\"jjwCr\"",
        )
    });
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_lazy_chunk_still_reaches_the_root_modules() {
    let (_, files) = unpacked(&HOIST.map(fixture)).expect("linked");
    let lazy = files
        .iter()
        .find(|(path, _)| path == "ibiaQ.js")
        .map(|(_, code)| code.as_str())
        .expect("lazy module");
    assert!(lazy.contains("require(\"./66COJ.js\")"), "{lazy}");
}

fn unloaded_chunk() -> String {
    fixture(SERVER[3])
        .replace("201,", "211,")
        .replace("202,", "212,")
        .replace("eval lib", "eval unloaded")
}

#[test]
fn the_turbopack_server_group_still_links() {
    assert_eq!(kind(&SERVER.map(fixture)), Some(BundleKind::Turbopack));
}

#[test]
fn a_turbopack_chunk_the_page_loader_never_loads_is_not_linked() {
    let mut sources = SERVER.map(fixture).to_vec();
    sources.push(unloaded_chunk());
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_turbopack_loader_that_loads_fewer_chunks_than_given_is_not_linked() {
    let mut sources = SERVER.map(fixture).to_vec();
    sources[0] = replaced(
        sources[0].clone(),
        "R.c(\"server/chunks/ssr/lib.js\");\n",
        "",
    );
    assert_eq!(kind(&sources), None);
}

#[test]
fn a_turbopack_loader_that_names_one_chunk_twice_is_not_linked() {
    let mut sources = SERVER.map(fixture).to_vec();
    sources[0] = replaced(
        sources[0].clone(),
        "R.c(\"server/chunks/ssr/lib.js\");",
        "R.c(\"server/chunks/ssr/app.js\");",
    );
    assert_eq!(kind(&sources), None);
}
