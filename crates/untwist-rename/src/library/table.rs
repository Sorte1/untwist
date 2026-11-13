use std::sync::{Arc, OnceLock, RwLock};

use rustc_hash::FxHashMap;

use super::hash::mix;

pub const HEADER: &str = "#untwist-library-table";
pub const FORMAT: &str = "v2";
const UNKNOWN: &str = "-";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Module,
    Wrapper,
    Export,
    Function,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Module => "module",
            Role::Wrapper => "wrapper",
            Role::Export => "export",
            Role::Function => "function",
        }
    }

    fn code(self) -> char {
        match self {
            Role::Module => 'm',
            Role::Wrapper => 'w',
            Role::Export => 'e',
            Role::Function => 'f',
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code {
            "m" => Some(Role::Module),
            "w" => Some(Role::Wrapper),
            "e" => Some(Role::Export),
            "f" => Some(Role::Function),
            _ => None,
        }
    }
}

pub fn loose_key(loose: (u32, u64)) -> u32 {
    (mix(loose.1 ^ u64::from(loose.0).rotate_left(40)) >> 32) as u32
}

pub fn exact_key(exact: u128) -> u64 {
    (exact >> 64) as u64 ^ exact as u64
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowData {
    pub loose: u32,
    pub exact: u64,
    pub label: String,
    pub versions: String,
    pub module: String,
    pub role: Role,
    pub own: Option<String>,
    pub inner: Vec<Option<String>>,
    pub outer: Vec<Option<String>>,
    pub specific: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub loose: u32,
    pub exact: u64,
    label: u32,
    versions: u32,
    module: u32,
    pub role: Role,
    own: u32,
    inner: u32,
    outer: u32,
    pub specific: u32,
}

#[derive(Default)]
struct Interner {
    strings: Vec<String>,
    index: FxHashMap<String, u32>,
    lists: Vec<Vec<u32>>,
    list_index: FxHashMap<Vec<u32>, u32>,
}

impl Interner {
    fn string(&mut self, text: &str) -> u32 {
        if let Some(&known) = self.index.get(text) {
            return known;
        }
        let id = u32::try_from(self.strings.len()).unwrap_or(u32::MAX);
        self.strings.push(text.to_string());
        self.index.insert(text.to_string(), id);
        id
    }

    fn name(&mut self, name: Option<&String>) -> u32 {
        self.string(name.map_or(UNKNOWN, String::as_str))
    }

    fn list(&mut self, names: &[Option<String>]) -> u32 {
        let ids: Vec<u32> = names.iter().map(|name| self.name(name.as_ref())).collect();
        if let Some(&known) = self.list_index.get(&ids) {
            return known;
        }
        let id = u32::try_from(self.lists.len()).unwrap_or(u32::MAX);
        self.lists.push(ids.clone());
        self.list_index.insert(ids, id);
        id
    }
}

#[derive(Clone, Default)]
pub struct Table {
    strings: Vec<String>,
    lists: Vec<Vec<u32>>,
    pub rows: Vec<Row>,
    loose: rustc_hash::FxHashSet<u32>,
    exact: FxHashMap<u64, (u32, u32)>,
}

impl Table {
    pub fn from_rows(rows: &[RowData]) -> Self {
        let mut interner = Interner::default();
        interner.string(UNKNOWN);
        let compact = rows
            .iter()
            .map(|row| Row {
                loose: row.loose,
                exact: row.exact,
                label: interner.string(&row.label),
                versions: interner.string(&row.versions),
                module: interner.string(&row.module),
                role: row.role,
                own: interner.name(row.own.as_ref()),
                inner: interner.list(&row.inner),
                outer: interner.list(&row.outer),
                specific: row.specific,
            })
            .collect();
        Self::assemble(interner.strings, interner.lists, compact)
    }

    fn assemble(strings: Vec<String>, lists: Vec<Vec<u32>>, mut rows: Vec<Row>) -> Self {
        rows.sort_by_key(|row| row.exact);
        let mut table = Table {
            strings,
            lists,
            rows,
            ..Table::default()
        };
        table.loose.reserve(table.rows.len());
        table.exact.reserve(table.rows.len());
        for (index, row) in table.rows.iter().enumerate() {
            table.loose.insert(row.loose);
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            table
                .exact
                .entry(row.exact)
                .and_modify(|(_, count)| *count += 1)
                .or_insert((index, 1));
        }
        table
    }

    pub fn merge(tables: Vec<Table>) -> Self {
        let mut strings: Vec<String> = Vec::new();
        let mut lists: Vec<Vec<u32>> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        for table in tables {
            let string_base = u32::try_from(strings.len()).unwrap_or(u32::MAX);
            let list_base = u32::try_from(lists.len()).unwrap_or(u32::MAX);
            strings.extend(table.strings);
            lists.extend(
                table
                    .lists
                    .into_iter()
                    .map(|list| list.into_iter().map(|id| id + string_base).collect()),
            );
            rows.extend(table.rows.into_iter().map(|row| Row {
                label: row.label + string_base,
                versions: row.versions + string_base,
                module: row.module + string_base,
                own: row.own + string_base,
                inner: row.inner + list_base,
                outer: row.outer + list_base,
                ..row
            }));
        }
        Self::assemble(strings, lists, rows)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn has_loose(&self, key: u32) -> bool {
        self.loose.contains(&key)
    }

    pub fn rows_for(&self, exact: u64) -> impl Iterator<Item = &Row> {
        let (start, count) = self.exact.get(&exact).copied().unwrap_or((0, 0));
        let start = start as usize;
        self.rows
            .get(start..start + count as usize)
            .unwrap_or_default()
            .iter()
    }

    pub fn text(&self, id: u32) -> &str {
        self.strings
            .get(id as usize)
            .map_or(UNKNOWN, String::as_str)
    }

    fn name(&self, id: u32) -> Option<&str> {
        let text = self.text(id);
        (text != UNKNOWN).then_some(text)
    }

    pub fn label(&self, row: &Row) -> &str {
        self.text(row.label)
    }

    pub fn versions(&self, row: &Row) -> &str {
        self.text(row.versions)
    }

    pub fn module(&self, row: &Row) -> &str {
        self.text(row.module)
    }

    pub fn own(&self, row: &Row) -> Option<&str> {
        self.name(row.own)
    }

    fn names(&self, list: u32) -> Vec<Option<&str>> {
        self.lists
            .get(list as usize)
            .map(|ids| ids.iter().map(|&id| self.name(id)).collect())
            .unwrap_or_default()
    }

    pub fn inner(&self, row: &Row) -> Vec<Option<&str>> {
        self.names(row.inner)
    }

    pub fn outer(&self, row: &Row) -> Vec<Option<&str>> {
        self.names(row.outer)
    }

    pub fn data(&self, row: &Row) -> RowData {
        let owned = |names: Vec<Option<&str>>| {
            names
                .into_iter()
                .map(|name| name.map(str::to_string))
                .collect()
        };
        RowData {
            loose: row.loose,
            exact: row.exact,
            label: self.label(row).to_string(),
            versions: self.versions(row).to_string(),
            module: self.module(row).to_string(),
            role: row.role,
            own: self.own(row).map(str::to_string),
            inner: owned(self.inner(row)),
            outer: owned(self.outer(row)),
            specific: row.specific,
        }
    }

    pub fn write(&self, id: &str) -> String {
        let mut text = format!(
            "{HEADER}\t{FORMAT}\t{id}\n#strings\t{}\n",
            self.strings.len()
        );
        for string in &self.strings {
            text.push_str(string);
            text.push('\n');
        }
        text.push_str(&format!("#lists\t{}\n", self.lists.len()));
        for list in &self.lists {
            let ids: Vec<String> = list.iter().map(u32::to_string).collect();
            text.push_str(&ids.join(" "));
            text.push('\n');
        }
        text.push_str(&format!("#rows\t{}\n", self.rows.len()));
        for row in &self.rows {
            text.push_str(&format!(
                "{:08x}\t{:016x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                row.loose,
                row.exact,
                row.label,
                row.versions,
                row.module,
                row.role.code(),
                row.own,
                row.inner,
                row.outer,
                row.specific
            ));
        }
        text
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let mut header = lines.next()?.split('\t');
        if header.next() != Some(HEADER) || header.next() != Some(FORMAT) {
            return None;
        }
        let count = |line: Option<&str>, tag: &str| -> Option<usize> {
            let (found, count) = line?.split_once('\t')?;
            (found == tag).then(|| count.parse().ok()).flatten()
        };
        let strings_count = count(lines.next(), "#strings")?;
        let strings: Vec<String> = lines
            .by_ref()
            .take(strings_count)
            .map(str::to_string)
            .collect();
        let lists_count = count(lines.next(), "#lists")?;
        let lists: Vec<Vec<u32>> = lines
            .by_ref()
            .take(lists_count)
            .map(|line| line.split(' ').filter_map(|id| id.parse().ok()).collect())
            .collect();
        let rows_count = count(lines.next(), "#rows")?;
        let mut rows = Vec::with_capacity(rows_count.min(text.len() / MIN_ROW_BYTES));
        for line in lines.take(rows_count) {
            rows.push(parse_row(line)?);
        }
        Some(Self::assemble(strings, lists, rows))
    }
}

fn parse_row(line: &str) -> Option<Row> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [loose, exact, label, versions, module, role, own, inner, outer, specific] =
        fields.as_slice()
    else {
        return None;
    };
    let number = |text: &str| -> Option<u32> { text.parse().ok() };
    Some(Row {
        loose: u32::from_str_radix(loose, 16).ok()?,
        exact: u64::from_str_radix(exact, 16).ok()?,
        label: number(label)?,
        versions: number(versions)?,
        module: number(module)?,
        role: Role::from_code(role)?,
        own: number(own)?,
        inner: number(inner)?,
        outer: number(outer)?,
        specific: number(specific)?,
    })
}

const MIN_ROW_BYTES: usize = 40;
pub const INFLATED_LIMIT: usize = 256 << 20;
const GZIP_HEADER: usize = 10;
const GZIP_TRAILER: usize = 8;

pub fn inflate(data: &[u8]) -> Option<String> {
    inflate_with_limit(data, INFLATED_LIMIT)
}

pub fn inflate_with_limit(data: &[u8], limit: usize) -> Option<String> {
    let body = data.get(GZIP_HEADER..data.len().checked_sub(GZIP_TRAILER)?)?;
    let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(body, limit).ok()?;
    String::from_utf8(raw).ok()
}

fn builtin_sources() -> Vec<(&'static str, &'static [u8])> {
    let mut sources: Vec<(&'static str, &'static [u8])> = Vec::new();
    #[cfg(feature = "libraries-core")]
    sources.extend_from_slice(untwist_libraries_core::TABLES);
    #[cfg(feature = "libraries-corejs")]
    sources.extend_from_slice(untwist_libraries_corejs::TABLES);
    #[cfg(feature = "libraries-polyfills")]
    sources.extend_from_slice(untwist_libraries_polyfills::TABLES);
    #[cfg(feature = "libraries-react")]
    sources.extend_from_slice(untwist_libraries_react::TABLES);
    #[cfg(feature = "libraries-frameworks")]
    sources.extend_from_slice(untwist_libraries_frameworks::TABLES);
    #[cfg(feature = "libraries-lodash")]
    sources.extend_from_slice(untwist_libraries_lodash::TABLES);
    #[cfg(feature = "libraries-dates")]
    sources.extend_from_slice(untwist_libraries_dates::TABLES);
    #[cfg(feature = "libraries-state")]
    sources.extend_from_slice(untwist_libraries_state::TABLES);
    #[cfg(feature = "libraries-data")]
    sources.extend_from_slice(untwist_libraries_data::TABLES);
    #[cfg(feature = "libraries-game")]
    sources.extend_from_slice(untwist_libraries_game::TABLES);
    #[cfg(feature = "libraries-pixi")]
    sources.extend_from_slice(untwist_libraries_pixi::TABLES);
    #[cfg(feature = "libraries-three")]
    sources.extend_from_slice(untwist_libraries_three::TABLES);
    #[cfg(feature = "libraries-pixi-legacy")]
    sources.extend_from_slice(untwist_libraries_pixi_legacy::TABLES);
    #[cfg(feature = "libraries-three-r100")]
    sources.extend_from_slice(untwist_libraries_three_r100::TABLES);
    #[cfg(feature = "libraries-three-r130")]
    sources.extend_from_slice(untwist_libraries_three_r130::TABLES);
    sources
}

pub struct LibraryTables {
    ids: Vec<String>,
    table: Table,
}

impl LibraryTables {
    pub fn from_gz_bytes(data: &[u8]) -> Option<Self> {
        Self::from_text(&inflate(data)?)
    }

    pub fn from_text(text: &str) -> Option<Self> {
        let id = text.lines().next()?.split('\t').nth(2)?.to_string();
        Some(Self {
            ids: vec![id],
            table: Table::parse(text)?,
        })
    }

    pub fn ids(&self) -> &[String] {
        &self.ids
    }
}

struct Loaded {
    ids: Vec<String>,
    table: Arc<Table>,
}

fn registry() -> &'static RwLock<Option<Loaded>> {
    static REGISTRY: OnceLock<RwLock<Option<Loaded>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(None))
}

fn builtin_loaded() -> Loaded {
    let mut ids = Vec::new();
    let mut tables = Vec::new();
    for (id, data) in builtin_sources() {
        if let Some(table) = inflate(data).and_then(|text| Table::parse(&text)) {
            ids.push(id.to_string());
            tables.push(table);
        }
    }
    let table = match tables.len() {
        0 => Table::default(),
        1 => tables.into_iter().next().unwrap_or_default(),
        _ => Table::merge(tables),
    };
    Loaded {
        ids,
        table: Arc::new(table),
    }
}

pub fn current() -> Arc<Table> {
    if let Some(loaded) = registry()
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|loaded| Arc::clone(&loaded.table)))
    {
        return loaded;
    }
    let mut guard = match registry().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    Arc::clone(&guard.get_or_insert_with(builtin_loaded).table)
}

pub fn load(extra: LibraryTables) -> bool {
    current();
    let mut guard = match registry().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let loaded = guard.get_or_insert_with(builtin_loaded);
    if extra.ids.iter().all(|id| loaded.ids.contains(id)) {
        return false;
    }
    let merged = Table::merge(vec![(*loaded.table).clone(), extra.table]);
    loaded.ids.extend(extra.ids);
    loaded.table = Arc::new(merged);
    true
}

pub fn loaded_ids() -> Vec<String> {
    current();
    registry()
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|loaded| loaded.ids.clone()))
        .unwrap_or_default()
}
