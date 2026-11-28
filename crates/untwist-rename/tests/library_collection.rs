use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;
use untwist_rename::library::collection::{
    bloom_file, parse_bloom_file, sha256_hex, shard_table, Bloom, Collection, CollectionError,
};
use untwist_rename::library::table::{inflate, Table};
use untwist_rename::library::SHAPE_EPOCH;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/library-collection")
}

fn index() -> String {
    std::fs::read_to_string(fixture().join("collection.json")).expect("the fixture index")
}

fn parsed() -> Collection {
    let mut collection = Collection::parse(&index()).expect("the fixture index parses");
    let filters = collection.filters().expect("a filter file").clone();
    let bytes = std::fs::read(fixture().join(&filters.file)).expect("the filter file");
    collection.attach_filters(&bytes).expect("the filters load");
    collection
}

fn shard_keys(collection: &Collection, package: usize) -> FxHashSet<u32> {
    let shard = &collection.packages()[package].shards[0];
    let bytes = std::fs::read(fixture().join(&shard.file)).expect("a shard");
    shard_table(shard, &bytes)
        .expect("the shard loads")
        .rows
        .iter()
        .map(|row| row.loose)
        .collect()
}

#[test]
fn the_fixture_collection_parses_and_names_its_packages() {
    let collection = parsed();
    let ids: Vec<&str> = collection
        .packages()
        .iter()
        .map(|package| package.id.as_str())
        .collect();
    assert_eq!(ids, ["eventemitter3", "events", "punycode"]);
    assert_eq!(collection.index.shape_epoch, SHAPE_EPOCH);
}

#[test]
fn a_package_bloom_holds_every_loose_key_of_its_shard() {
    let collection = parsed();
    for package in 0..collection.packages().len() {
        let keys = shard_keys(&collection, package);
        for key in &keys {
            let one: FxHashSet<u32> = [*key].into_iter().collect();
            assert!(collection.wanted_by_keys(&one).contains(&package));
        }
    }
}

#[test]
fn unrelated_keys_want_almost_no_package() {
    let collection = parsed();
    let known: FxHashSet<u32> = (0..collection.packages().len())
        .flat_map(|package| shard_keys(&collection, package))
        .collect();
    let probes = (0u32..20_000)
        .map(|seed| seed.wrapping_mul(2_654_435_761))
        .filter(|key| !known.contains(key));
    let mut hits = 0;
    let mut total = 0;
    for key in probes {
        total += 1;
        let one: FxHashSet<u32> = [key].into_iter().collect();
        hits += collection.wanted_by_keys(&one).len();
    }
    assert!(
        hits * 100 < total * 3,
        "{hits} false hits in {total} probes"
    );
}

#[test]
fn another_shape_epoch_is_refused() {
    let text = index().replacen(
        &format!("\"shape_epoch\": {SHAPE_EPOCH}"),
        &format!("\"shape_epoch\": {}", SHAPE_EPOCH + 1),
        1,
    );
    assert_eq!(
        Collection::parse(&text).unwrap_err(),
        CollectionError::Epoch {
            found: SHAPE_EPOCH + 1
        }
    );
}

#[test]
fn another_schema_and_broken_json_are_refused() {
    let text = index().replacen("library-collection/1", "library-collection/9", 1);
    assert!(matches!(
        Collection::parse(&text),
        Err(CollectionError::Schema(_))
    ));
    assert!(matches!(
        Collection::parse("{"),
        Err(CollectionError::Json(_))
    ));
}

#[test]
fn a_shard_path_that_leaves_the_collection_is_refused() {
    let collection = parsed();
    let file = collection.packages()[0].shards[0].file.clone();
    for bad in ["../outside.utl", "/etc/passwd", "shards/../../x.utl"] {
        let text = index().replacen(&file, bad, 1);
        assert!(
            matches!(Collection::parse(&text), Err(CollectionError::Shard { .. })),
            "{bad}"
        );
    }
}

#[test]
fn a_shard_whose_bytes_differ_from_the_index_is_refused() {
    let collection = parsed();
    let shard = &collection.packages()[0].shards[0];
    let mut bytes = std::fs::read(fixture().join(&shard.file)).expect("a shard");
    let last = bytes.len() - 9;
    bytes[last] ^= 1;
    assert!(matches!(
        shard_table(shard, &bytes),
        Err(CollectionError::Shard { .. })
    ));
}

#[test]
fn filters_whose_hash_differs_are_refused() {
    let mut collection = Collection::parse(&index()).expect("the fixture index parses");
    assert!(matches!(
        collection.attach_filters(b"UTBLOOM1\0\0\0\0"),
        Err(CollectionError::Filters(_))
    ));
}

#[test]
fn a_bloom_file_round_trips() {
    let entries = vec![
        ("a".to_string(), Bloom::with_keys(&[1, 2, 3])),
        ("@scope/b".to_string(), Bloom::with_keys(&[])),
    ];
    let bytes = bloom_file(&entries);
    assert_eq!(parse_bloom_file(&bytes), Some(entries));
    assert_eq!(parse_bloom_file(&bytes[..bytes.len() - 1]), None);
    assert_eq!(sha256_hex(b"").len(), 64);
}

#[test]
fn every_fixture_shard_is_a_v2_table_named_by_its_hash() {
    let collection = parsed();
    for shard in collection
        .packages()
        .iter()
        .flat_map(|package| &package.shards)
    {
        let bytes = std::fs::read(fixture().join(&shard.file)).expect("a shard");
        assert!(shard.file.contains(&sha256_hex(&bytes)));
        let text = inflate(&bytes).expect("gzip");
        assert!(Table::parse(&text).is_some());
    }
}

#[test]
fn a_shard_bloom_selects_only_its_own_shard() {
    use untwist_rename::library::collection::shard_bloom_id;
    let shard = |hex: char| {
        format!(
            "{{\"file\": \"shards/{0}.utl\", \"bytes\": 1, \"sha256\": \"{0}\"}}",
            hex.to_string().repeat(64)
        )
    };
    let blooms = bloom_file(&[
        (shard_bloom_id("three", 0), Bloom::with_keys(&[11])),
        (shard_bloom_id("three", 1), Bloom::with_keys(&[22])),
    ]);
    let sha = sha256_hex(&blooms);
    let index = format!(
        "{{\"schema\": \"untwist.library-collection/1\", \"version\": \"t\", \"shape_epoch\": {SHAPE_EPOCH}, \"packages\": [{{\"id\": \"three\", \"label\": \"three\", \"shards\": [{}, {}]}}], \"filters\": {{\"file\": \"shards/{sha}.bloom\", \"sha256\": \"{sha}\"}}}}",
        shard('a'),
        shard('b')
    );
    let mut collection = Collection::parse(&index).expect("the index parses");
    collection
        .attach_filters(&blooms)
        .expect("the filters load");
    let keys = |key: u32| -> FxHashSet<u32> { [key].into_iter().collect() };
    assert_eq!(collection.shards_wanted_by_keys(&keys(11)), [(0, 0)]);
    assert_eq!(collection.shards_wanted_by_keys(&keys(22)), [(0, 1)]);
    assert_eq!(collection.wanted_by_keys(&keys(22)), [0]);
    assert_eq!(collection.shard_count(), 2);
}

#[test]
fn a_generated_package_is_wanted_only_on_enough_evidence() {
    use untwist_rename::library::collection::ProgramKeys;
    use untwist_rename::library::CLUSTER_HITS;
    let collection = parsed();
    let keys: Vec<u32> = shard_keys(&collection, 0).into_iter().collect();
    let some = |count: usize, program: Option<u32>| ProgramKeys {
        loose: keys.iter().take(count).copied().collect(),
        strong: keys.iter().take(count.min(1)).copied().collect(),
        program,
        pairs: Vec::new(),
    };
    let wanted = |found: ProgramKeys| -> Vec<usize> {
        collection
            .shards_wanted(&found)
            .into_iter()
            .map(|(package, _)| package)
            .collect()
    };
    assert!(!wanted(some(CLUSTER_HITS - 1, None)).contains(&0));
    assert!(wanted(some(CLUSTER_HITS, None)).contains(&0));
    assert!(wanted(some(0, Some(keys[0]))).contains(&0));
}

#[test]
fn an_unrelated_big_program_wants_almost_no_package() {
    use untwist_rename::library::collection::ProgramKeys;
    let collection = parsed();
    let known: FxHashSet<u32> = (0..collection.packages().len())
        .flat_map(|package| shard_keys(&collection, package))
        .collect();
    let loose: FxHashSet<u32> = (0u32..300)
        .map(|seed| seed.wrapping_mul(2_654_435_761))
        .filter(|key| !known.contains(key))
        .collect();
    let found = ProgramKeys {
        strong: loose.clone(),
        loose,
        program: None,
        pairs: Vec::new(),
    };
    let wanted = collection.shards_wanted(&found);
    assert!(wanted.len() <= 1, "{wanted:?}");
}

#[test]
fn the_fixture_filter_file_is_what_bloom_file_writes_for_its_shards() {
    let collection = parsed();
    let entries: Vec<(String, Bloom)> = (0..collection.packages().len())
        .map(|package| {
            let keys: Vec<u32> = shard_keys(&collection, package).into_iter().collect();
            (
                collection.packages()[package].id.clone(),
                Bloom::with_keys(&keys),
            )
        })
        .collect();
    let filters = collection.filters().expect("a filter file");
    let written = std::fs::read(fixture().join(&filters.file)).expect("the filter file");
    assert_eq!(bloom_file(&entries), written);
}

#[test]
fn a_curated_package_needs_the_same_evidence_as_a_generated_one() {
    use untwist_rename::library::collection::ProgramKeys;
    use untwist_rename::library::CLUSTER_HITS;
    let keys: Vec<u32> = (1u32..=8)
        .map(|seed| seed.wrapping_mul(2_654_435_761))
        .collect();
    let blooms = bloom_file(&[("core-js".to_string(), Bloom::with_keys(&keys))]);
    let sha = sha256_hex(&blooms);
    let shard = format!(
        "{{\"file\": \"shards/{0}.utl\", \"bytes\": 1, \"sha256\": \"{0}\"}}",
        "c".repeat(64)
    );
    let index = format!(
        "{{\"schema\": \"untwist.library-collection/1\", \"version\": \"t\", \"shape_epoch\": {SHAPE_EPOCH}, \"packages\": [{{\"id\": \"core-js\", \"label\": \"core-js\", \"curated\": true, \"shards\": [{shard}]}}], \"filters\": {{\"file\": \"shards/{sha}.bloom\", \"sha256\": \"{sha}\"}}}}"
    );
    let mut collection = Collection::parse(&index).expect("the index parses");
    collection
        .attach_filters(&blooms)
        .expect("the filters load");
    let found = |count: usize, strong: usize| ProgramKeys {
        loose: keys.iter().take(count).copied().collect(),
        strong: keys.iter().take(strong).copied().collect(),
        program: None,
        pairs: Vec::new(),
    };
    assert!(collection.shards_wanted(&found(1, 1)).is_empty());
    assert!(collection.shards_wanted(&found(CLUSTER_HITS, 0)).is_empty());
    assert_eq!(collection.shards_wanted(&found(CLUSTER_HITS, 1)), [(0, 0)]);
}

fn shard_pairs(collection: &Collection, package: usize) -> Vec<(u32, u64)> {
    let shard = &collection.packages()[package].shards[0];
    let bytes = std::fs::read(fixture().join(&shard.file)).expect("a shard");
    shard_table(shard, &bytes)
        .expect("the shard loads")
        .rows
        .iter()
        .map(|row| (row.loose, row.exact))
        .collect()
}

fn with_exact_filters(collection: &Collection) -> Collection {
    use untwist_rename::library::collection::exact_bloom_id;
    let entries: Vec<(String, Bloom)> = (0..collection.packages().len())
        .flat_map(|package| {
            let id = collection.packages()[package].id.clone();
            let pairs = shard_pairs(collection, package);
            let loose: Vec<u32> = pairs.iter().map(|(loose, _)| *loose).collect();
            [
                (id.clone(), Bloom::with_keys(&loose)),
                (exact_bloom_id(&id), Bloom::with_pairs(&pairs)),
            ]
        })
        .collect();
    let bytes = bloom_file(&entries);
    let sha = sha256_hex(&bytes);
    let mut index = collection.index.clone();
    index.filters = Some(untwist_rename::library::collection::FileRef {
        file: format!("shards/{sha}.bloom"),
        sha256: sha,
        bytes: bytes.len() as u64,
    });
    let json = serde_json::to_string(&index).expect("the index serialises");
    let mut exact = Collection::parse(&json).expect("the index parses");
    exact.attach_filters(&bytes).expect("the filters load");
    exact
}

#[test]
fn an_exact_filter_drops_a_shard_whose_loose_hits_have_no_exact_twin() {
    use untwist_rename::library::collection::ProgramKeys;
    let collection = with_exact_filters(&parsed());
    assert!(collection.has_exact_filters());
    assert!(!parsed().has_exact_filters());
    let pairs = shard_pairs(&collection, 0);
    let found = |pairs: Vec<(u32, u64)>| ProgramKeys {
        loose: pairs.iter().map(|(loose, _)| *loose).collect(),
        strong: pairs.iter().map(|(loose, _)| *loose).collect(),
        program: None,
        pairs,
    };
    let packages = |keys: &ProgramKeys| -> Vec<usize> {
        collection
            .shards_wanted(keys)
            .into_iter()
            .map(|(package, _)| package)
            .collect()
    };
    assert!(packages(&found(pairs.clone())).contains(&0));
    let twisted: Vec<(u32, u64)> = pairs
        .iter()
        .map(|&(loose, exact)| (loose, !exact))
        .collect();
    assert!(!packages(&found(twisted.clone())).contains(&0));
    assert!(parsed()
        .shards_wanted(&found(twisted))
        .iter()
        .any(|(package, _)| *package == 0));
}

#[test]
fn an_exact_bloom_entry_is_kept_apart_from_the_loose_one() {
    use untwist_rename::library::collection::{exact_bloom_id, pair_key};
    let pairs = [(7u32, 99u64), (8, 100)];
    let bloom = Bloom::with_pairs(&pairs);
    assert!(pairs
        .iter()
        .all(|&(loose, exact)| bloom.contains_pair(loose, exact)));
    assert_ne!(pair_key(7, 99), pair_key(99, 7));
    assert_eq!(exact_bloom_id("three#1"), "three#1!");
    let file = bloom_file(&[(exact_bloom_id("three#1"), bloom.clone())]);
    assert_eq!(
        parse_bloom_file(&file).expect("the file parses"),
        [(exact_bloom_id("three#1"), bloom)]
    );
}
