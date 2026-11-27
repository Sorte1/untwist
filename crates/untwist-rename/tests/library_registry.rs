use std::path::{Path, PathBuf};

use untwist_rename::library::collection::Collection;
use untwist_rename::library::registry::{attach, load, load_shard, use_builtin};
use untwist_rename::library::table::{current, inflate, LibraryTables, Table};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/library-collection")
}

fn collection() -> (Collection, Vec<u8>) {
    let index = std::fs::read_to_string(fixture().join("collection.json")).expect("the index");
    let collection = Collection::parse(&index).expect("the index parses");
    let shard = collection.packages()[0].shards[0].file.clone();
    let bytes = std::fs::read(fixture().join(shard)).expect("a shard");
    (collection, bytes)
}

fn same_rows_as_a_table(shard: &[u8]) -> LibraryTables {
    let text = inflate(shard).expect("gzip");
    let table = Table::parse(&text).expect("the shard parses");
    LibraryTables::from_text(&table.write("copy-of-a-shard")).expect("a table")
}

fn loaded(shard_first: bool) -> String {
    let (collection, bytes) = collection();
    use_builtin(false);
    attach(collection, None);
    match shard_first {
        true => {
            load_shard(&bytes).expect("the shard loads");
            load(same_rows_as_a_table(&bytes));
        }
        false => {
            load(same_rows_as_a_table(&bytes));
            load_shard(&bytes).expect("the shard loads");
        }
    }
    current().write("all")
}

#[test]
fn the_loaded_rows_do_not_depend_on_whether_a_table_or_a_shard_came_first() {
    let shard_first = loaded(true);
    let table_first = loaded(false);
    assert_eq!(shard_first, table_first);
}
