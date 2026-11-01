use indexmap::IndexMap;
use rustc_hash::FxHashSet;

use crate::model::Bundle;
use crate::path::{dirname_posix, join_posix};

pub type DependencyTree = IndexMap<String, Vec<(String, String)>>;

const NODE_MODULES: &str = "node_modules";
const ENTRY_PATH: &str = "index.js";
const TMP_PREFIX: &str = "tmp";

pub struct Resolution {
    pub paths: IndexMap<String, String>,
    pub extensionless: FxHashSet<String>,
}

struct Frame {
    node: String,
    cwd: String,
    index: usize,
}

pub fn dependency_tree(bundle: &Bundle<'_>) -> DependencyTree {
    bundle
        .modules
        .iter()
        .map(|(id, module)| (id.clone(), module.dependencies.clone()))
        .collect()
}

fn resolve_request(cwd: &str, request: &str, index_dir: bool) -> (String, bool) {
    if !request.starts_with('.') {
        return (
            join_posix(NODE_MODULES, &format!("{request}/{ENTRY_PATH}")),
            false,
        );
    }
    let path = join_posix(cwd, request);
    if path.ends_with(".js") {
        return (path, false);
    }
    if index_dir {
        (join_posix(&path, ENTRY_PATH), true)
    } else {
        (format!("{path}.js"), true)
    }
}

pub fn resolve_raw(
    tree: &DependencyTree,
    entry: &str,
    index_dirs: &FxHashSet<String>,
) -> Resolution {
    let mut paths: IndexMap<String, String> = IndexMap::new();
    let mut extensionless: FxHashSet<String> = FxHashSet::default();
    let mut stack = vec![Frame {
        node: entry.to_string(),
        cwd: String::new(),
        index: 0,
    }];
    while let Some(frame) = stack.last_mut() {
        let node = frame.node.clone();
        let cwd = frame.cwd.clone();
        let index = frame.index;
        frame.index += 1;
        let next = tree.get(&node).and_then(|deps| deps.get(index)).cloned();
        let Some((dep_id, request)) = next else {
            stack.pop();
            continue;
        };
        if paths.contains_key(&dep_id) {
            continue;
        }
        let (path, appended) = resolve_request(&cwd, &request, index_dirs.contains(&dep_id));
        if appended {
            extensionless.insert(dep_id.clone());
        }
        let next_cwd = dirname_posix(&path).to_string();
        paths.insert(dep_id.clone(), path);
        stack.push(Frame {
            node: dep_id,
            cwd: next_cwd,
            index: 0,
        });
    }
    Resolution {
        paths,
        extensionless,
    }
}

fn entry_prefix(paths: &IndexMap<String, String>) -> String {
    let depth = paths
        .values()
        .map(|path| path.matches("..").count() + 1)
        .max()
        .unwrap_or(1);
    (0..depth.saturating_sub(1))
        .map(|index| format!("{TMP_PREFIX}{index}"))
        .collect::<Vec<String>>()
        .join("/")
}

pub fn resolve_dependency_tree(
    tree: &DependencyTree,
    entry: &str,
    index_dirs: &FxHashSet<String>,
) -> IndexMap<String, String> {
    let mut paths = resolve_raw(tree, entry, index_dirs).paths;
    paths.insert(entry.to_string(), ENTRY_PATH.to_string());
    let prefix = entry_prefix(&paths);
    paths
        .into_iter()
        .map(|(id, path)| {
            if path.starts_with(&format!("{NODE_MODULES}/")) {
                (id, path)
            } else {
                (id, join_posix(&prefix, &path))
            }
        })
        .collect()
}

const MAX_REPAIR_ROUNDS: usize = 8;

fn escapes(cwd: &str, request: &str) -> bool {
    if !request.starts_with('.') {
        return false;
    }
    let joined = join_posix(cwd, request);
    joined == ".." || joined.starts_with("../")
}

fn next_flip(
    tree: &DependencyTree,
    resolution: &Resolution,
    index_dirs: &FxHashSet<String>,
) -> Option<String> {
    for (id, path) in &resolution.paths {
        if index_dirs.contains(id) || !resolution.extensionless.contains(id) {
            continue;
        }
        let Some(stem) = path.strip_suffix(".js") else {
            continue;
        };
        let Some(requests) = tree.get(id) else {
            continue;
        };
        let current = dirname_posix(path);
        let escaping_now = requests
            .iter()
            .any(|(_, request)| escapes(current, request));
        let escaping_alt = requests.iter().any(|(_, request)| escapes(stem, request));
        if escaping_now && !escaping_alt {
            return Some(id.clone());
        }
    }
    None
}

pub fn infer_index_directories(tree: &DependencyTree, entry: &str) -> FxHashSet<String> {
    let mut index_dirs: FxHashSet<String> = FxHashSet::default();
    for _ in 0..MAX_REPAIR_ROUNDS {
        let resolution = resolve_raw(tree, entry, &index_dirs);
        let Some(flip) = next_flip(tree, &resolution, &index_dirs) else {
            break;
        };
        index_dirs.insert(flip);
    }
    index_dirs
}
