use std::path::{Path, PathBuf};

use untwist::libraries::{self, Collection, Shard, ShardSource, INDEX};
use untwist::{Options, Pipeline};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/library-collection")
}

struct Directory(PathBuf);

impl ShardSource for Directory {
    fn read(&self, shard: &Shard) -> Result<Vec<u8>, String> {
        std::fs::read(self.0.join(&shard.file)).map_err(|error| error.to_string())
    }
}

fn library_names(source: &str) -> Vec<String> {
    Pipeline::new(Options {
        unsafe_transforms: true,
        ..Options::default()
    })
    .run(source)
    .libraries()
    .into_iter()
    .map(|summary| summary.library)
    .collect()
}

#[test]
fn an_attached_collection_loads_only_the_shards_its_blooms_want() {
    let source =
        std::fs::read_to_string(fixture().join("inputs/eventemitter3.rollup-cjs.terser.js"))
            .expect("the fixture input");
    assert!(!library_names(&source).contains(&"eventemitter3".to_string()));
    let mut collection =
        Collection::parse(&std::fs::read_to_string(fixture().join(INDEX)).expect("the index"))
            .expect("the index parses");
    let filters = collection.filters().expect("filters").clone();
    collection
        .attach_filters(&std::fs::read(fixture().join(&filters.file)).expect("the filters"))
        .expect("the filters load");
    libraries::attach(collection, Some(Box::new(Directory(fixture()))));
    let found = library_names(&source);
    assert!(found.contains(&"eventemitter3".to_string()), "{found:?}");
    let status = libraries::collection_status().expect("a collection is attached");
    assert_eq!(status.shards_loaded, ["eventemitter3"]);
    assert!(status.failures.is_empty(), "{:?}", status.failures);
    assert_eq!(library_names(&source), found);
}
