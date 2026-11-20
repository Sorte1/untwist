use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::run;
use untwist_testing::node_rows::{node_or_skip, observed};

const DATA: &str = "tests/data/shared_inputs";

const UNSAFE_PREMISE: &[&str] = &[
    "fin-f1",
    "fin-f1call",
    "fin-f1n",
    "fin-f1own",
    "fin-f1s",
    "fin-f1x",
    "fin-f2",
    "fin-f2own",
    "fin-f3",
    "fin-f3e",
    "fin-f3n",
    "fin-f3two",
    "fin-f5",
    "fin-f5g",
    "fin-f5rot",
    "fin-f5two",
    "fin-s1",
    "fin-s2",
    "ml8-args",
    "ml8-arr",
    "ml8-arrow",
    "ml8-await",
    "ml8-catch",
    "ml8-cls",
    "ml8-deep",
    "ml8-defp",
    "ml8-destr",
    "ml8-forof",
    "ml8-g",
    "ml8-gen",
    "ml8-logic",
    "ml8-nw",
    "ml8-obj",
    "ml8-rcons",
    "ml8-ret",
    "ml8-seq",
    "ml8-spread",
    "ml8-tern",
    "ml8-then",
    "ml8-vari",
    "ml8-win",
    "tri-A_ck_abc",
    "tri-A_key_abc",
    "tri-A_keyt_abc",
    "tri-A_keyv_abc",
    "tri-D_key_abc",
    "tri-D_keyt_abc",
    "tri-D_keyv_abc",
    "tri-PA_ck_abc",
    "tri-PA_holdk_abc",
    "tri-PA_holdl_abc",
    "tri-PA_holdp_abc",
    "tri-PA_holdv_abc",
    "tri-PA_holdw_abc",
    "tri-PA_key_abc",
    "tri-PA_keyt_abc",
    "tri-PA_keyv_abc",
    "tri-P_ck_abc",
    "tri-P_holdk_abc",
    "tri-P_holdl_abc",
    "tri-P_holdp_abc",
    "tri-P_holdv_abc",
    "tri-P_holdw_abc",
    "tri-P_key_abc",
    "tri-P_keyt_abc",
    "tri-P_keyv_abc",
    "tri2-N_cfg_abc",
    "tri2-N_cfgr_abc",
    "tri2-N_hask_abc",
    "tri2-N_ks_abc",
    "tri2-N_ksr_abc",
    "tri2-N_mc_two",
    "tri2-N_oeach_abc",
    "tri2-N_okeys_abc",
    "tri2-N_olet_two",
    "tri2-N_owr_two",
    "tri2-N_setk_two",
    "tri2-N_setkt_two",
    "tri2-N_thisk_abc",
    "tri2-N_wink_abc",
    "tri2-P_F_two",
    "tri2-P_Fd_two",
    "tri2-P_Fg_two",
    "tri2-P_Fr_two",
    "tri2-P_Frt_two",
    "tri2-P_arr_two",
    "tri2-P_cfg_abc",
    "tri2-P_cfgr_abc",
    "tri2-P_g_two",
    "tri2-P_hask_abc",
    "tri2-P_ks_abc",
    "tri2-P_ksr_abc",
    "tri2-P_m_two",
    "tri2-P_mc_two",
    "tri2-P_mk_two",
    "tri2-P_mkd_two",
    "tri2-P_ns_two",
    "tri2-P_nsg_two",
    "tri2-P_nsr_two",
    "tri2-P_o_two",
    "tri2-P_ocall_two",
    "tri2-P_oeach_abc",
    "tri2-P_of_two",
    "tri2-P_og_two",
    "tri2-P_oin_two",
    "tri2-P_okeys_abc",
    "tri2-P_olet_two",
    "tri2-P_ov_two",
    "tri2-P_owr_two",
    "tri2-P_setk_two",
    "tri2-P_setkt_two",
    "tri2-P_thisk_abc",
    "tri2-P_wink_abc",
    "triN-A_ck_abc",
    "triN-A_holdk_abc",
    "triN-A_holdl_abc",
    "triN-A_holdp_abc",
    "triN-A_holdv_abc",
    "triN-A_holdw_abc",
    "triN-A_key_abc",
    "triN-A_keyt_abc",
    "triN-A_keyv_abc",
    "triN-D_holdk_abc",
    "triN-D_key_abc",
    "triN-D_keyt_abc",
    "triN-D_keyv_abc",
];

const GLOBAL_NAMES: [&str; 5] = ["window", "self", "frames", "top", "parent"];

struct Case {
    label: String,
    inputs: Vec<(String, String)>,
}

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(DATA)
}

fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths
}

fn file_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().to_string()
}

fn cases() -> Vec<Case> {
    sorted_entries(&data())
        .into_iter()
        .map(|dir| Case {
            label: file_name(&dir),
            inputs: sorted_entries(&dir)
                .into_iter()
                .map(|path| (file_name(&path), std::fs::read_to_string(&path).unwrap()))
                .collect(),
        })
        .collect()
}

fn runner(names: &[String]) -> String {
    let mut text = String::from(
        "import fs from \"fs\"; import vm from \"vm\"; import { pathToFileURL } from \"url\"; import path from \"path\";\nconst here = path.dirname(new URL(import.meta.url).pathname);\n",
    );
    for name in GLOBAL_NAMES {
        text.push_str(&format!(
            "Object.defineProperty(globalThis, \"{name}\", {{ value: globalThis, writable: true, configurable: true, enumerable: false }});\n"
        ));
    }
    text.push_str(
        "process.on(\"uncaughtException\", (e) => console.log(\"UNCAUGHT\", e && e.name));\nprocess.on(\"unhandledRejection\", (e) => console.log(\"UNHANDLED\", e && e.name));\n",
    );
    for name in names {
        let quoted = serde_json::to_string(name).unwrap();
        let load = match name.ends_with(".mjs") {
            true => format!("await import(pathToFileURL(path.join(here, {quoted})).href);"),
            false => format!(
                "vm.runInThisContext(fs.readFileSync(path.join(here, {quoted}), \"utf8\"), {{ importModuleDynamically: vm.constants.USE_MAIN_CONTEXT_DEFAULT_LOADER }});"
            ),
        };
        text.push_str(&format!(
            "try {{ {load} }} catch (e) {{ console.log(\"THROW\", e && e.name); }}\n"
        ));
    }
    text
}

fn printed(node: &Path, dir: &Path, files: &[(String, String)]) -> String {
    std::fs::create_dir_all(dir).unwrap();
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    let main = dir.join("run.mjs");
    std::fs::write(&main, runner(&names)).unwrap();
    match run(node, &main).unwrap() {
        Some(out) => observed(&out),
        None => "did not finish".to_string(),
    }
}

fn rows(unsafe_transforms: bool) -> Vec<String> {
    let Some(node) = node_or_skip() else {
        return Vec::new();
    };
    let temp = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for case in cases() {
        if unsafe_transforms && UNSAFE_PREMISE.contains(&case.label.as_str()) {
            continue;
        }
        let base = temp.path().join(&case.label);
        let before = printed(&node, &base.join("in"), &case.inputs);
        let sources: Vec<&str> = case.inputs.iter().map(|(_, text)| text.as_str()).collect();
        let options = Options {
            unsafe_transforms,
            unpack: false,
            ..Options::default()
        };
        let codes = Pipeline::new(options).run_many(&sources).codes;
        let outputs: Vec<(String, String)> = case
            .inputs
            .iter()
            .zip(codes)
            .map(|((name, _), code)| (name.clone(), code))
            .collect();
        let after = printed(&node, &base.join("out"), &outputs);
        if after != before {
            wrong.push(format!(
                "{} (unsafe={unsafe_transforms}): node printed {before:?}, the outputs printed {after:?}",
                case.label
            ));
        }
    }
    wrong
}

#[test]
fn inputs_sharing_one_global_scope_print_the_same_in_node_in_the_default_lane() {
    let wrong = rows(false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn inputs_sharing_one_global_scope_outside_the_unsafe_premise_print_the_same_under_unsafe() {
    let wrong = rows(true);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
