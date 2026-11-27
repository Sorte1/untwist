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
    collection.attach_filters(&blooms).expect("the filters load");
    let keys = |key: u32| -> FxHashSet<u32> { [key].into_iter().collect() };
    assert_eq!(collection.shards_wanted_by_keys(&keys(11)), [(0, 0)]);
    assert_eq!(collection.shards_wanted_by_keys(&keys(22)), [(0, 1)]);
    assert_eq!(collection.wanted_by_keys(&keys(22)), [0]);
    assert_eq!(collection.shard_count(), 2);
}
