use serde::Serialize;

use crate::options::TypesMode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TypeCounts {
    pub eligible: usize,
    pub proven: usize,
    pub annotated: usize,
    pub redundant: usize,
    pub refused: usize,
}

impl TypeCounts {
    pub fn absorb(&mut self, other: TypeCounts) {
        self.eligible += other.eligible;
        self.proven += other.proven;
        self.annotated += other.annotated;
        self.redundant += other.redundant;
        self.refused += other.refused;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TypesReport {
    pub mode: &'static str,
    pub bindings: TypeCounts,
    pub params: TypeCounts,
    pub returns: TypeCounts,
    pub files: usize,
    pub files_refused: usize,
    pub near_eval: usize,
    pub elapsed_ms: f64,
    #[serde(skip)]
    pub typescript: bool,
    #[serde(skip)]
    pub jsx_outputs: Vec<bool>,
    #[serde(skip)]
    pub ts_outputs: Vec<bool>,
    #[serde(skip)]
    pub ts_modules: Vec<bool>,
    #[serde(skip)]
    pub jsx_modules: Vec<bool>,
    #[serde(skip)]
    pub declarations: Vec<Option<String>>,
}

impl TypesReport {
    pub fn new(mode: TypesMode) -> Self {
        Self {
            mode: mode.as_str(),
            bindings: TypeCounts::default(),
            params: TypeCounts::default(),
            returns: TypeCounts::default(),
            files: 0,
            files_refused: 0,
            near_eval: 0,
            elapsed_ms: 0.0,
            typescript: mode == TypesMode::Ts,
            jsx_outputs: Vec::new(),
            ts_outputs: Vec::new(),
            ts_modules: Vec::new(),
            jsx_modules: Vec::new(),
            declarations: Vec::new(),
        }
    }

    pub fn total(&self) -> TypeCounts {
        let mut total = self.bindings;
        total.absorb(self.params);
        total.absorb(self.returns);
        total
    }

    pub fn line(&self) -> String {
        let total = self.total();
        format!(
            "types ({}): {} annotated ({} of them tsc infers anyway), {} refused of {} proven ({} positions; bindings {}/{}, params {}/{}, returns {}/{}); {} of {} files refused; {:.1} ms\n",
            self.mode,
            total.annotated,
            total.redundant,
            total.refused,
            total.proven,
            total.eligible,
            self.bindings.annotated,
            self.bindings.eligible,
            self.params.annotated,
            self.params.eligible,
            self.returns.annotated,
            self.returns.eligible,
            self.files_refused,
            self.files,
            self.elapsed_ms,
        )
    }
}

const TS_EXTENSIONS: [(&str, &str, &str); 4] = [
    (".js", ".ts", ".tsx"),
    (".mjs", ".mts", ".mts"),
    (".cjs", ".cts", ".cts"),
    (".jsx", ".tsx", ".tsx"),
];

pub fn typed_path(path: &str, jsx: bool) -> String {
    for (from, plain, with_jsx) in TS_EXTENSIONS {
        if let Some(stem) = path.strip_suffix(from) {
            let to = if jsx { with_jsx } else { plain };
            return format!("{stem}{to}");
        }
    }
    match jsx {
        true => format!("{path}.tsx"),
        false => format!("{path}.ts"),
    }
}

fn normalized_path(dir: &str, specifier: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in specifier.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

fn directory_of(path: &str) -> &str {
    path.rfind('/').map_or("", |at| &path[..at])
}

fn relative(dir: &str, to: &str) -> String {
    let from: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    let target: Vec<&str> = to.split('/').filter(|part| !part.is_empty()).collect();
    let shared = from.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<&str> = vec![".."; from.len() - shared];
    parts.extend(&target[shared..]);
    let joined = parts.join("/");
    match joined.starts_with("../") {
        true => joined,
        false => format!("./{joined}"),
    }
}

const RESOLVED_SUFFIXES: [&str; 6] = ["", ".js", ".mjs", ".cjs", ".jsx", "/index.js"];

type Found = Vec<(u32, u32, Option<String>)>;

fn retargeted(
    code: &str,
    found: &Found,
    dir: &str,
    renames: &std::collections::HashMap<String, String>,
) -> String {
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (start, end, value) in found {
        let Some(value) = value else {
            continue;
        };
        if !(value.starts_with("./") || value.starts_with("../")) {
            continue;
        }
        let Some(base) = normalized_path(dir, value) else {
            continue;
        };
        let target = RESOLVED_SUFFIXES
            .iter()
            .find_map(|suffix| renames.get(&format!("{base}{suffix}")));
        if let Some(target) = target {
            let quote = code
                .as_bytes()
                .get(*start as usize)
                .copied()
                .unwrap_or(b'"') as char;
            let text = relative(dir, target);
            edits.push((
                *start as usize,
                *end as usize,
                format!("{quote}{text}{quote}"),
            ));
        }
    }
    let mut out = code.to_string();
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    for (start, end, text) in edits {
        out.replace_range(start..end, &text);
    }
    out
}

pub(crate) struct Untyped {
    pub codes: Vec<String>,
    pub modules: Vec<String>,
}

fn restore_modules(output: &mut crate::Output, report: &mut TypesReport, untyped: &Untyped) {
    if let Some(bundle) = output.bundle.as_mut() {
        for (index, module) in bundle.modules.iter_mut().enumerate() {
            if report.ts_modules.get(index).copied().unwrap_or(false) {
                if let Some(text) = untyped.modules.get(index) {
                    module.code = text.clone();
                }
                report.ts_modules[index] = false;
            }
        }
    }
}

fn restore_codes(output: &mut crate::Output, report: &mut TypesReport, untyped: &Untyped) {
    for (index, code) in output.codes.iter_mut().enumerate() {
        if report.ts_outputs.get(index).copied().unwrap_or(false) {
            if let Some(text) = untyped.codes.get(index) {
                *code = text.clone();
            }
            report.ts_outputs[index] = false;
        }
    }
}

pub(crate) fn settle_names(
    output: &mut crate::Output,
    report: &mut TypesReport,
    untyped: &Untyped,
    notes: &mut Vec<untwist_core::pass::Note>,
) {
    if !report.typescript {
        return;
    }
    if output.single_file {
        let whole = report
            .ts_outputs
            .iter()
            .chain(&report.ts_modules)
            .all(|ts| *ts);
        if !whole {
            restore_modules(output, report, untyped);
            restore_codes(output, report, untyped);
        }
        return;
    }
    let Some(bundle) = output.bundle.as_mut() else {
        return;
    };
    let mut found: Vec<Option<Found>> = Vec::with_capacity(bundle.modules.len());
    for (index, module) in bundle.modules.iter_mut().enumerate() {
        let typescript = report.ts_modules.get(index).copied().unwrap_or(false);
        let read = crate::pipeline::specifier_list(&module.code, typescript);
        if read.is_none() && typescript {
            if let Some(text) = untyped.modules.get(index) {
                module.code = text.clone();
            }
            report.ts_modules[index] = false;
            found.push(crate::pipeline::specifier_list(&module.code, false));
            continue;
        }
        found.push(read);
    }
    let code_found: Vec<Option<Found>> = output
        .codes
        .iter()
        .enumerate()
        .map(|(index, code)| {
            let typescript = report.ts_outputs.get(index).copied().unwrap_or(false);
            crate::pipeline::specifier_list(code, typescript)
        })
        .collect();
    let computed = found.iter().chain(&code_found).any(|read| {
        read.as_ref()
            .is_none_or(|list| list.iter().any(|(_, _, value)| value.is_none()))
    });
    if computed {
        if report.ts_modules.iter().any(|ts| *ts) {
            crate::pipeline::kept_names_note(notes);
        }
        restore_modules(output, report, untyped);
        return;
    }
    let renames: std::collections::HashMap<String, String> = bundle
        .modules
        .iter()
        .enumerate()
        .filter(|(index, _)| report.ts_modules.get(*index).copied().unwrap_or(false))
        .map(|(index, module)| {
            let jsx = report.jsx_modules.get(index).copied().unwrap_or(false);
            (
                normalized_path("", &module.path).unwrap_or_default(),
                typed_path(&module.path, jsx),
            )
        })
        .collect();
    if renames.is_empty() {
        return;
    }
    for (code, read) in output.codes.iter_mut().zip(&code_found) {
        if let Some(read) = read {
            *code = retargeted(code, read, "", &renames);
        }
    }
    for (index, (module, read)) in bundle.modules.iter_mut().zip(&found).enumerate() {
        if let Some(read) = read {
            module.code = retargeted(&module.code, read, directory_of(&module.path), &renames);
        }
        if report.ts_modules.get(index).copied().unwrap_or(false) {
            let jsx = report.jsx_modules.get(index).copied().unwrap_or(false);
            module.path = typed_path(&module.path, jsx);
        }
    }
}

const DECLARATION_EXTENSIONS: [(&str, &str); 3] =
    [(".js", ".d.ts"), (".mjs", ".d.mts"), (".cjs", ".d.cts")];

pub fn declaration_path(path: &str) -> String {
    for (from, to) in DECLARATION_EXTENSIONS {
        if let Some(stem) = path.strip_suffix(from) {
            return format!("{stem}{to}");
        }
    }
    format!("{path}.d.ts")
}

#[cfg(test)]
mod tests {
    use super::{declaration_path, typed_path};

    #[test]
    fn declaration_files_sit_beside_their_module() {
        assert_eq!(declaration_path("src/a.js"), "src/a.d.ts");
        assert_eq!(declaration_path("a.mjs"), "a.d.mts");
        assert_eq!(declaration_path("modules/12"), "modules/12.d.ts");
    }

    #[test]
    fn typescript_paths_follow_the_javascript_extension() {
        assert_eq!(typed_path("deobfuscated.js", false), "deobfuscated.ts");
        assert_eq!(typed_path("src/app.js", true), "src/app.tsx");
        assert_eq!(typed_path("a.mjs", false), "a.mts");
        assert_eq!(typed_path("a.jsx", false), "a.tsx");
        assert_eq!(typed_path("modules/12", false), "modules/12.ts");
    }
}
