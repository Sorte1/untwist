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

pub const MAX_SEGMENT_BYTES: usize = 255;
pub const MAX_PATH_BYTES: usize = 1024;
pub const RESERVED_FILES: &[&str] = &["deobfuscated.js", "bundle.json"];
pub const RESERVED_DIRECTORIES: &[&str] = &["wasm"];

const FORBIDDEN_CHARACTERS: &[char] = &[':', '<', '>', '"', '|', '?', '*', '\\'];
const TOOL_DIRECTORIES: &[&str] = &[
    ".git", ".hg", ".svn", ".vscode", ".idea", ".husky", ".github",
];
const DEVICE_NAMES: &[&str] = &[
    "con",
    "prn",
    "aux",
    "nul",
    "com1",
    "com2",
    "com3",
    "com4",
    "com5",
    "com6",
    "com7",
    "com8",
    "com9",
    "lpt1",
    "lpt2",
    "lpt3",
    "lpt4",
    "lpt5",
    "lpt6",
    "lpt7",
    "lpt8",
    "lpt9",
    "com\u{b9}",
    "com\u{b2}",
    "com\u{b3}",
    "lpt\u{b9}",
    "lpt\u{b2}",
    "lpt\u{b3}",
    "conin$",
    "conout$",
];

fn is_unprintable(ch: char) -> bool {
    ch.is_control()
        || matches!(
            ch,
            '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
        )
}

fn is_short_name(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment);
    stem.rsplit_once('~')
        .is_some_and(|(_, tail)| !tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_device_name(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or(segment)
        .trim_end_matches(' ');
    DEVICE_NAMES
        .iter()
        .any(|name| stem.eq_ignore_ascii_case(name))
}

fn is_unsafe_segment(segment: &str) -> bool {
    segment.len() > MAX_SEGMENT_BYTES
        || segment.ends_with(['.', ' '])
        || is_device_name(segment)
        || is_short_name(segment)
        || TOOL_DIRECTORIES
            .iter()
            .any(|name| segment.eq_ignore_ascii_case(name))
}

pub fn is_unsafe_path(normalized: &str) -> bool {
    if normalized.is_empty() || normalized.len() > MAX_PATH_BYTES || normalized.starts_with('/') {
        return true;
    }
    if normalized == ".." || normalized.starts_with("../") {
        return true;
    }
    if normalized
        .chars()
        .any(|ch| is_unprintable(ch) || FORBIDDEN_CHARACTERS.contains(&ch))
    {
        return true;
    }
    if normalized.split('/').any(is_unsafe_segment) {
        return true;
    }
    normalized
        .rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with('.'))
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

pub(crate) fn relative_posix(from: &str, to: &str) -> String {
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
