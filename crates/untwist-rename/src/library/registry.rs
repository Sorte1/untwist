use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use rustc_hash::FxHashSet;

use super::collection::{sha256_hex, shard_table, Collection, CollectionError, ProgramKeys, Shard};
use super::table::{builtin_tables, LibraryTables, Table};

pub trait ShardSource: Send + Sync {
    fn read(&self, shard: &Shard) -> Result<Vec<u8>, String>;
}

struct Attached {
    collection: Collection,
    source: Option<Box<dyn ShardSource>>,
    loaded: BTreeMap<(usize, usize), Table>,
    curated: BTreeSet<(usize, usize)>,
    tried: FxHashSet<(usize, usize)>,
    failures: Vec<String>,
}

impl Attached {
    fn pending(&self) -> bool {
        self.source.is_some() && self.tried.len() < self.collection.shard_count()
    }

    fn loaded_packages(&self) -> Vec<String> {
        let mut packages: Vec<usize> = self
            .loaded
            .keys()
            .map(|(package, _)| *package)
            .chain(self.curated.iter().map(|(package, _)| *package))
            .collect();
        packages.sort_unstable();
        packages.dedup();
        packages
            .into_iter()
            .map(|package| self.collection.packages()[package].id.clone())
            .collect()
    }

    fn has_loaded(&self, package: usize) -> bool {
        self.loaded
            .keys()
            .chain(&self.curated)
            .any(|(known, _)| *known == package)
    }

    fn has_shard(&self, slot: (usize, usize)) -> bool {
        self.loaded.contains_key(&slot) || self.curated.contains(&slot)
    }

    fn built_in(&self, package: usize, built_in: &[String]) -> bool {
        let entry = &self.collection.packages()[package];
        entry.curated && built_in.contains(&entry.id) && !self.has_loaded(package)
    }

    fn fetch(&mut self, (package, index): (usize, usize), built_in: &[String]) -> Option<Table> {
        self.tried.insert((package, index));
        let source = self.source.as_ref()?;
        if self.built_in(package, built_in) {
            return None;
        }
        let entry = &self.collection.packages()[package];
        let curated = entry.curated;
        let shard = entry.shards[index].clone();
        let table = source
            .read(&shard)
            .map_err(|reason| CollectionError::Shard {
                file: shard.file.clone(),
                reason,
            })
            .and_then(|bytes| shard_table(&shard, &bytes));
        match (table, curated) {
            (Ok(table), true) => {
                self.curated.insert((package, index));
                Some(table)
            }
            (Ok(table), false) => {
                self.loaded.insert((package, index), table);
                None
            }
            (Err(error), _) => {
                self.failures.push(error.to_string());
                None
            }
        }
    }
}

struct Loaded {
    ids: Vec<String>,
    base: Arc<Table>,
    table: Arc<Table>,
    attached: Option<Attached>,
}

impl Loaded {
    fn with_base(ids: Vec<String>, base: Table) -> Self {
        let base = Arc::new(base);
        Self {
            ids,
            table: Arc::clone(&base),
            base,
            attached: None,
        }
    }

    fn adopt(&mut self, id: String, tables: Vec<Table>) {
        if tables.is_empty() {
            return;
        }
        self.base = Arc::new(Table::merge(
            std::iter::once((*self.base).clone())
                .chain(tables)
                .collect(),
        ));
        if !self.ids.contains(&id) {
            self.ids.push(id);
        }
    }

    fn rebuild(&mut self) {
        let shards: Vec<Table> = self
            .attached
            .iter()
            .flat_map(|attached| attached.loaded.values())
            .map(|table| table.clone().into_collection(&self.base))
            .collect();
        self.table = match shards.is_empty() {
            true => Arc::clone(&self.base),
            false => Arc::new(Table::merge(
                std::iter::once((*self.base).clone())
                    .chain(shards)
                    .collect(),
            )),
        };
    }
}

fn registry() -> &'static RwLock<Option<Loaded>> {
    static REGISTRY: OnceLock<RwLock<Option<Loaded>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(None))
}

fn builtin_loaded() -> Loaded {
    let (ids, table) = builtin_tables();
    Loaded::with_base(ids, table)
}

fn read() -> RwLockReadGuard<'static, Option<Loaded>> {
    match registry().read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write() -> RwLockWriteGuard<'static, Option<Loaded>> {
    match registry().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn with_loaded<T>(change: impl FnOnce(&mut Loaded) -> T) -> T {
    let mut guard = write();
    change(guard.get_or_insert_with(builtin_loaded))
}

pub fn current() -> Arc<Table> {
    if let Some(table) = read().as_ref().map(|loaded| Arc::clone(&loaded.table)) {
        return table;
    }
    with_loaded(|loaded| Arc::clone(&loaded.table))
}

pub fn use_builtin(enabled: bool) {
    let loaded = match enabled {
        true => builtin_loaded(),
        false => Loaded::with_base(Vec::new(), Table::default()),
    };
    *write() = Some(loaded);
}

pub fn load(extra: LibraryTables) -> bool {
    with_loaded(|loaded| {
        if extra.ids().iter().all(|id| loaded.ids.contains(id)) {
            return false;
        }
        let (ids, table) = extra.into_parts();
        loaded.base = Arc::new(Table::merge(vec![(*loaded.base).clone(), table]));
        loaded.ids.extend(ids);
        loaded.rebuild();
        true
    })
}

pub fn loaded_ids() -> Vec<String> {
    with_loaded(|loaded| {
        let mut ids = loaded.ids.clone();
        if let Some(attached) = &loaded.attached {
            ids.extend(attached.loaded_packages());
        }
        ids
    })
}

pub fn attach(collection: Collection, source: Option<Box<dyn ShardSource>>) {
    with_loaded(|loaded| {
        loaded.attached = Some(Attached {
            collection,
            source,
            loaded: BTreeMap::new(),
            curated: BTreeSet::new(),
            tried: FxHashSet::default(),
            failures: Vec::new(),
        });
        loaded.rebuild();
    });
}

pub fn pending() -> bool {
    read()
        .as_ref()
        .and_then(|loaded| loaded.attached.as_ref())
        .is_some_and(Attached::pending)
}

pub fn wanting(keys: &ProgramKeys) -> Arc<Table> {
    with_loaded(|loaded| {
        let Some(attached) = loaded.attached.as_mut() else {
            return Arc::clone(&loaded.table);
        };
        let wanted: Vec<(usize, usize)> = attached
            .collection
            .shards_wanted(keys)
            .into_iter()
            .filter(|slot| !attached.tried.contains(slot))
            .collect();
        if wanted.is_empty() {
            return Arc::clone(&loaded.table);
        }
        let before = attached.loaded.len();
        let mut adopted = Vec::new();
        for slot in wanted {
            if let Some(table) = attached.fetch(slot, &loaded.ids) {
                let id = attached.collection.packages()[slot.0].id.clone();
                adopted.push((id, vec![table]));
            }
        }
        let changed = attached.loaded.len() != before || !adopted.is_empty();
        for (id, tables) in adopted {
            loaded.adopt(id, tables);
        }
        if changed {
            loaded.rebuild();
        }
        Arc::clone(&loaded.table)
    })
}

pub fn wanted(text: &str, keys: &ProgramKeys) -> Vec<String> {
    with_loaded(|loaded| {
        let Some(attached) = &loaded.attached else {
            return Vec::new();
        };
        let collection = &attached.collection;
        let mut slots: Vec<(usize, usize)> = collection
            .wanted_by_text(text)
            .into_iter()
            .flat_map(|package| {
                (0..collection.packages()[package].shards.len()).map(move |shard| (package, shard))
            })
            .collect();
        slots.extend(collection.shards_wanted(keys));
        slots.sort_unstable();
        slots.dedup();
        slots
            .into_iter()
            .filter(|&slot| !attached.has_shard(slot) && !attached.built_in(slot.0, &loaded.ids))
            .map(|(package, shard)| collection.packages()[package].shards[shard].sha256.clone())
            .collect()
    })
}

pub fn load_shard(bytes: &[u8]) -> Result<String, CollectionError> {
    with_loaded(|loaded| {
        let attached = loaded
            .attached
            .as_mut()
            .ok_or_else(|| CollectionError::Unknown(sha256_hex(bytes)))?;
        let sha = sha256_hex(bytes);
        let (package, shard) = attached
            .collection
            .shard_owner(&sha)
            .ok_or(CollectionError::Unknown(sha))?;
        let index = attached.collection.packages()[package]
            .shards
            .iter()
            .position(|known| known == shard)
            .unwrap_or_default();
        let table = shard_table(shard, bytes)?;
        let id = attached.collection.packages()[package].id.clone();
        if attached.collection.packages()[package].curated {
            let built_in = attached.built_in(package, &loaded.ids);
            if built_in || !attached.curated.insert((package, index)) {
                return Ok(id);
            }
            loaded.adopt(id.clone(), vec![table]);
            loaded.rebuild();
            return Ok(id);
        }
        let fresh = attached.loaded.insert((package, index), table).is_none();
        if fresh {
            loaded.rebuild();
        }
        Ok(id)
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CollectionStatus {
    pub version: String,
    pub epoch: u32,
    pub packages: usize,
    pub shards_loaded: Vec<String>,
    pub failures: Vec<String>,
}

pub fn collection_status() -> Option<CollectionStatus> {
    read()
        .as_ref()
        .and_then(|loaded| loaded.attached.as_ref())
        .map(|attached| CollectionStatus {
            version: attached.collection.index.version.clone(),
            epoch: attached.collection.index.shape_epoch,
            packages: attached.collection.packages().len(),
            shards_loaded: attached.loaded_packages(),
            failures: attached.failures.clone(),
        })
}

pub fn provides(id: &str) -> bool {
    with_loaded(|loaded| {
        loaded.ids.iter().any(|known| known == id)
            || loaded.attached.as_ref().is_some_and(|attached| {
                attached.source.is_some()
                    && attached
                        .collection
                        .packages()
                        .iter()
                        .any(|package| package.id == id)
            })
    })
}
