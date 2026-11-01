const NODE_MODULES: &str = "node_modules/";

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PosixPath(String);

pub fn normalize_posix(raw: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut leading_parents = 0usize;
    for segment in raw.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if out.pop().is_none() {
                    leading_parents += 1;
                }
            }
            other => out.push(other),
        }
    }
    let mut parts: Vec<&str> = Vec::with_capacity(leading_parents + out.len());
    parts.extend(std::iter::repeat_n("..", leading_parents));
    parts.extend(out);
    parts.join("/")
}

pub fn is_unsafe_path(normalized: &str) -> bool {
    if normalized.is_empty() {
        return true;
    }
    if normalized.starts_with('/') || normalized.contains('\\') {
        return true;
    }
    if normalized
        .split('/')
        .next()
        .is_some_and(|first| first.len() >= 2 && first.as_bytes()[1] == b':')
    {
        return true;
    }
    normalized == ".." || normalized.starts_with("../")
}

pub fn join_posix(base: &str, rest: &str) -> String {
    if base.is_empty() {
        return normalize_posix(rest);
    }
    if rest.is_empty() {
        return normalize_posix(base);
    }
    normalize_posix(&format!("{base}/{rest}"))
}

pub fn relative_path(from: &str, to: &str) -> String {
    let to = normalize_posix(to);
    if let Some(bare) = to.strip_prefix(NODE_MODULES) {
        return bare.to_string();
    }
    let relative = relative_posix(dirname_of(from), &to);
    if relative.starts_with('.') {
        relative
    } else {
        format!("./{relative}")
    }
}

fn relative_posix(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let shared = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<&str> = vec![".."; from_parts.len() - shared];
    parts.extend(to_parts.iter().skip(shared).copied());
    parts.join("/")
}

fn dirname_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(index) => &path[..index],
        None => "",
    }
}

pub fn dirname_posix(path: &str) -> &str {
    dirname_of(path)
}

pub const ENTRY_PATH: &str = "index.js";

pub fn module_id_path(id: &str, is_entry: bool) -> String {
    if is_entry {
        return ENTRY_PATH.to_string();
    }
    let stem = id.strip_suffix(".js").unwrap_or(id);
    format!("{stem}.js")
}

impl PosixPath {
    pub fn parse(raw: &str) -> Option<Self> {
        let normalized = normalize_posix(raw);
        if is_unsafe_path(&normalized) {
            return None;
        }
        Some(Self(normalized))
    }

    pub fn from_module_id(id: &str, is_entry: bool) -> Option<Self> {
        Self::parse(&module_id_path(id, is_entry))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn dirname(&self) -> &str {
        dirname_of(&self.0)
    }

    pub fn is_node_modules(&self) -> bool {
        self.0.starts_with(NODE_MODULES)
    }

    pub fn with_stem_suffix(&self, suffix: &str) -> Self {
        match self.0.rfind('.') {
            Some(dot) if dot > self.0.rfind('/').map_or(0, |slash| slash + 1) => {
                Self(format!("{}{suffix}{}", &self.0[..dot], &self.0[dot..]))
            }
            _ => Self(format!("{}{suffix}", self.0)),
        }
    }
}
