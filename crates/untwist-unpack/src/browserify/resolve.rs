use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::number_key;
use untwist_core::host_globals::NODE_BUILTIN_MODULES;

use crate::browserify::requests::package_name;
use crate::model::{Bundle, Module, RequireParam};
use crate::path::{dirname_posix, join_posix, RESERVED_FILES};

const NODE_SCHEME: &str = "node:";
const NODE_MODULES: &str = "node_modules";
const FILE_EXTENSIONS: [&str; 4] = ["", ".js", ".json", ".node"];
const INDEX_FILES: [&str; 3] = ["index.js", "index.json", "index.node"];
const PACKAGE_JSON: &str = "package.json";
const MAP_SUFFIX: &str = ".map";
const REQUIRE: &str = "require";

pub struct Reached {
    pub id: String,
    pub request: Option<String>,
    pub path: String,
}

struct Written {
    files: FxHashSet<String>,
}

impl Written {
    fn of(bundle: &Bundle<'_>) -> Self {
        let modules = bundle
            .modules
            .values()
            .map(|module| module.path.as_str().to_string());
        let maps = bundle
            .modules
            .values()
            .map(|module| format!("{}{MAP_SUFFIX}", module.path.as_str()));
        let reserved = RESERVED_FILES.iter().map(|name| name.to_string());
        let mirrored = bundle.mirrored_inputs.iter().cloned();
        let files = modules
            .chain(maps)
            .chain(reserved)
            .chain(mirrored)
            .map(|path| path.to_lowercase())
            .collect();
        Self { files }
    }

    fn holds(&self, path: &str) -> bool {
        self.files.contains(&path.to_lowercase())
    }

    fn answer(&self, base: &str) -> Option<String> {
        let as_file = FILE_EXTENSIONS
            .iter()
            .map(|extension| format!("{base}{extension}"));
        let as_directory = INDEX_FILES
            .iter()
            .chain([&PACKAGE_JSON])
            .map(|name| join_posix(base, name));
        as_file.chain(as_directory).find(|path| self.holds(path))
    }
}

fn is_builtin(request: &str) -> bool {
    request.starts_with(NODE_SCHEME) || NODE_BUILTIN_MODULES.contains(&request)
}

fn is_relative(request: &str) -> bool {
    request == "." || request == ".." || request.starts_with("./") || request.starts_with("../")
}

fn outside_the_tree(path: &str) -> bool {
    path == ".." || path.starts_with("../")
}

fn ancestors(directory: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current = directory.to_string();
    loop {
        found.push(current.clone());
        if current.is_empty() {
            return found;
        }
        current = dirname_posix(&current).to_string();
    }
}

fn package_bases(directory: &str, request: &str) -> Vec<String> {
    ancestors(directory)
        .into_iter()
        .filter(|ancestor| ancestor.rsplit('/').next() != Some(NODE_MODULES))
        .map(|ancestor| join_posix(&join_posix(&ancestor, NODE_MODULES), request))
        .collect()
}

fn resolved(written: &Written, from: &str, request: &str) -> Option<String> {
    if is_builtin(request) {
        return None;
    }
    if request.starts_with('/') || request.contains('\\') {
        return Some(request.to_string());
    }
    let directory = dirname_posix(from);
    if is_relative(request) {
        let base = join_posix(directory, request);
        if outside_the_tree(&base) {
            return None;
        }
        return written.answer(&base);
    }
    package_bases(directory, request)
        .iter()
        .find_map(|base| written.answer(base))
}

pub fn left_request_reaching(bundle: &Bundle<'_>) -> Option<Reached> {
    let written = Written::of(bundle);
    bundle.left_requests.iter().find_map(|(id, request)| {
        let from = bundle.path_of(id)?.as_str();
        let path = match request {
            Some(request) => resolved(&written, from, request)?,
            None => from.to_string(),
        };
        Some(Reached {
            id: id.clone(),
            request: request.clone(),
            path,
        })
    })
}

struct Unmapped<'m> {
    name: &'m str,
    module: &'m Module<'m>,
    ids: &'m FxHashSet<&'m str>,
    packages: FxHashSet<String>,
}

impl Unmapped<'_> {
    fn left(&self, request: &str) -> bool {
        !self.ids.contains(request)
            && !self
                .module
                .dependencies
                .iter()
                .any(|(_, mapped)| mapped == request)
    }
}

impl<'a> VisitJs<'a> for Unmapped<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let named =
            matches!(&it.callee, Expression::Identifier(callee) if callee.name == self.name);
        let request = match it.arguments.first() {
            Some(Argument::StringLiteral(literal)) => literal.value.as_str().map(str::to_string),
            Some(Argument::NumericLiteral(literal)) => Some(number_key(literal.value)),
            _ => None,
        };
        if let Some(request) = request.filter(|request| named && self.left(request)) {
            self.packages
                .extend(package_name(&request).map(str::to_lowercase));
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub fn left_packages(bundle: &Bundle<'_>) -> FxHashSet<String> {
    let ids: FxHashSet<&str> = bundle.modules.keys().map(String::as_str).collect();
    let mut packages = FxHashSet::default();
    for (id, module) in &bundle.modules {
        if module.runtime_require == RequireParam::Absent {
            continue;
        }
        let name = bundle
            .fresh_requires
            .iter()
            .find(|(fresh, _)| fresh == id)
            .map_or(REQUIRE, |(_, name)| name.as_str());
        let mut unmapped = Unmapped {
            name,
            module,
            ids: &ids,
            packages: FxHashSet::default(),
        };
        unmapped.visit_program(module.program());
        packages.extend(unmapped.packages);
    }
    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(paths: &[&str]) -> Written {
        Written {
            files: paths.iter().map(|path| path.to_lowercase()).collect(),
        }
    }

    #[test]
    fn a_package_request_walks_every_node_modules_directory_up_to_the_root() {
        let tree = written(&["index.js", "node_modules/foo/index.js"]);
        assert_eq!(
            resolved(&tree, "a/b/c.js", "foo").as_deref(),
            Some("node_modules/foo/index.js")
        );
        assert_eq!(
            resolved(&tree, "index.js", "foo/index").as_deref(),
            Some("node_modules/foo/index.js")
        );
        assert_eq!(resolved(&tree, "index.js", "bar"), None);
    }

    #[test]
    fn a_file_next_to_node_modules_answers_with_an_extension() {
        let tree = written(&["index.js", "node_modules/foo.js", "node_modules/bar.json"]);
        assert_eq!(
            resolved(&tree, "index.js", "foo").as_deref(),
            Some("node_modules/foo.js")
        );
        assert_eq!(
            resolved(&tree, "index.js", "bar").as_deref(),
            Some("node_modules/bar.json")
        );
    }

    #[test]
    fn a_nested_node_modules_directory_is_not_searched_twice() {
        let tree = written(&["node_modules/node_modules/foo.js"]);
        assert_eq!(resolved(&tree, "node_modules/a.js", "foo"), None);
        let tree = written(&["node_modules/a/node_modules/foo.js"]);
        assert_eq!(
            resolved(&tree, "node_modules/a/b.js", "foo").as_deref(),
            Some("node_modules/a/node_modules/foo.js")
        );
    }

    #[test]
    fn built_ins_never_reach_the_tree() {
        let tree = written(&["node_modules/fs/index.js", "node_modules/node:fs.js"]);
        assert_eq!(resolved(&tree, "index.js", "fs"), None);
        assert_eq!(resolved(&tree, "index.js", "node:fs"), None);
    }

    #[test]
    fn relative_and_absolute_requests_resolve_or_refuse() {
        let tree = written(&["index.js", "lib/a.js", "deobfuscated.js"]);
        assert_eq!(
            resolved(&tree, "index.js", "./lib/a").as_deref(),
            Some("lib/a.js")
        );
        assert_eq!(
            resolved(&tree, "lib/a.js", "../deobfuscated").as_deref(),
            Some("deobfuscated.js")
        );
        assert_eq!(resolved(&tree, "index.js", "../x"), None);
        assert!(resolved(&tree, "index.js", "/abs").is_some());
    }

    #[test]
    fn written_paths_are_compared_without_case() {
        let tree = written(&["node_modules/Foo/index.js"]);
        assert!(resolved(&tree, "index.js", "foo").is_some());
    }
}
