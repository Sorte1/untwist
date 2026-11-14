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
    pub elapsed_ms: f64,
    #[serde(skip)]
    pub typescript: bool,
    #[serde(skip)]
    pub jsx_outputs: Vec<bool>,
    #[serde(skip)]
    pub jsx_modules: Vec<bool>,
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
            elapsed_ms: 0.0,
            typescript: mode == TypesMode::Ts,
            jsx_outputs: Vec::new(),
            jsx_modules: Vec::new(),
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
            "types ({}): {} annotated, {} redundant, {} refused of {} proven ({} positions; bindings {}/{}, params {}/{}, returns {}/{}); {} of {} files refused; {:.1} ms\n",
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

#[cfg(test)]
mod tests {
    use super::typed_path;

    #[test]
    fn typescript_paths_follow_the_javascript_extension() {
        assert_eq!(typed_path("deobfuscated.js", false), "deobfuscated.ts");
        assert_eq!(typed_path("src/app.js", true), "src/app.tsx");
        assert_eq!(typed_path("a.mjs", false), "a.mts");
        assert_eq!(typed_path("a.jsx", false), "a.tsx");
        assert_eq!(typed_path("modules/12", false), "modules/12.ts");
    }
}
