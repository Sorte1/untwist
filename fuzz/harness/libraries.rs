use untwist::libraries::{self, sha256_hex, Collection, Shard, ShardSource, SCHEMA, SHAPE_EPOCH};

const SEPARATOR: &str = "\n%%\n";
const BLOOM_MAGIC: &[u8] = b"UTBLOOM1";
const GZIP_HEADER: [u8; 10] = [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 2, 0xff];
const GZIP_TRAILER: [u8; 8] = [0; 8];
const FILTERS_FILE: &str = "shards/filters.bloom";

struct Memory {
    files: Vec<(String, Vec<u8>)>,
}

impl ShardSource for Memory {
    fn read(&self, shard: &Shard) -> Result<Vec<u8>, String> {
        self.files
            .iter()
            .find(|(file, _)| *file == shard.file)
            .map(|(_, bytes)| bytes.clone())
            .ok_or_else(|| "missing".to_string())
    }
}

fn gzip(text: &str) -> Vec<u8> {
    let mut out = GZIP_HEADER.to_vec();
    out.extend(miniz_oxide::deflate::compress_to_vec(text.as_bytes(), 1));
    out.extend(GZIP_TRAILER);
    out
}

fn shard_json(file: &str, bytes: &[u8]) -> serde_json::Value {
    serde_json::json!({
        "file": file,
        "bytes": bytes.len(),
        "sha256": sha256_hex(bytes),
    })
}

struct Parts<'a> {
    flags: u8,
    source: String,
    tables: Vec<String>,
    keys: String,
    bloom: &'a [u8],
    index: Option<String>,
}

fn split<'a>(mut bytes: &'a [u8], separator: &[u8]) -> Vec<&'a [u8]> {
    let mut out = Vec::new();
    while let Some(at) = bytes
        .windows(separator.len())
        .position(|window| window == separator)
    {
        out.push(&bytes[..at]);
        bytes = &bytes[at + separator.len()..];
    }
    out.push(bytes);
    out
}

fn text(bytes: Option<&&[u8]>) -> String {
    String::from_utf8_lossy(bytes.copied().unwrap_or_default()).into_owned()
}

fn parts(data: &[u8]) -> Option<Parts<'_>> {
    let (&flags, rest) = data.split_first()?;
    let pieces = split(rest, SEPARATOR.as_bytes());
    let tables = text(pieces.get(1));
    Some(Parts {
        flags,
        source: text(pieces.first()),
        tables: tables
            .split("\n&&\n")
            .filter(|table| !table.is_empty())
            .map(str::to_string)
            .collect(),
        keys: text(pieces.get(2)),
        bloom: pieces.get(3).copied().unwrap_or_default(),
        index: pieces.get(4).map(|raw| text(Some(raw))),
    })
}

fn collection(parts: &Parts) -> (String, Memory) {
    let mut files = Vec::new();
    let shards: Vec<serde_json::Value> = parts
        .tables
        .iter()
        .enumerate()
        .map(|(index, table)| {
            let bytes = gzip(table);
            let file = format!("shards/{index}.utl");
            let json = shard_json(&file, &bytes);
            files.push((file, bytes));
            json
        })
        .collect();
    let keys: Vec<&str> = parts.keys.lines().collect();
    let mut index = serde_json::json!({
        "schema": SCHEMA,
        "version": "fuzz",
        "shape_epoch": SHAPE_EPOCH,
        "packages": [{
            "id": "fuzzed",
            "label": "fuzzed",
            "shards": shards,
            "keys": keys,
            "curated": parts.flags & 1 != 0,
        }],
    });
    if !parts.bloom.is_empty() {
        let mut bloom = BLOOM_MAGIC.to_vec();
        bloom.extend_from_slice(parts.bloom);
        index["filters"] = shard_json(FILTERS_FILE, &bloom);
        files.push((FILTERS_FILE.to_string(), bloom));
    }
    let json = match &parts.index {
        Some(raw) => raw.clone(),
        None => index.to_string(),
    };
    (json, Memory { files })
}

fn attach(json: &str, memory: Memory) {
    let Ok(mut parsed) = Collection::parse(json) else {
        return;
    };
    if let Some(filters) = parsed.filters().cloned() {
        if let Ok(bytes) = memory.read(&Shard {
            file: filters.file,
            bytes: 0,
            sha256: String::new(),
            versions: String::new(),
            rows: 0,
        }) {
            let _ = parsed.attach_filters(&bytes);
        }
    }
    libraries::attach(parsed, Some(Box::new(memory)));
}

pub fn run(data: &[u8]) {
    let Some(parts) = parts(data) else {
        return;
    };
    for table in &parts.tables {
        let _ = libraries::LibraryTables::from_text(table);
    }
    libraries::use_builtin(parts.flags & 2 != 0);
    let (json, memory) = collection(&parts);
    let shard_bytes: Vec<Vec<u8>> = memory
        .files
        .iter()
        .map(|(_, bytes)| bytes.clone())
        .collect();
    attach(&json, memory);
    if parts.flags & 4 != 0 {
        let _ = libraries::wanted_for(&parts.source);
        for bytes in &shard_bytes {
            let _ = libraries::load_shard(bytes);
        }
    }
    let options = untwist::Options {
        unsafe_transforms: parts.flags & 8 != 0,
        library_imports: parts.flags & 16 != 0,
        commonjs_goal: parts.flags & 32 != 0,
        max_iterations: Some(8),
        ..untwist::Options::default()
    };
    let out = untwist::Pipeline::new(options).run(&parts.source);
    assert!(
        out.panicked.is_empty(),
        "the pipeline panicked: {:?}",
        out.panicked
    );
    let _ = out.libraries();
    let _ = libraries::collection_status();
    let _ = libraries::loaded_ids();
}
