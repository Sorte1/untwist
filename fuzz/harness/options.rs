use untwist::{Options, Pipeline, RenameMode, TypesMode};

const RENAMES: [RenameMode; 5] = [
    RenameMode::None,
    RenameMode::Mangle,
    RenameMode::Simple,
    RenameMode::Smart,
    RenameMode::Unique,
];
const BASELINES: [&str; 3] = ["es2017", "es2022", "chrome 120, firefox 121, safari 17"];
const PLAN_DIR: &str = "/nonexistent/untwist-fuzz-out";
const SOURCE_MAP_SEPARATOR: &str = "\n//# fuzz-map\n";

fn bit(flags: u32, index: u32) -> bool {
    flags & (1 << index) != 0
}

fn options_for(flags: u32, input_map: Option<String>) -> Options {
    let goal = (flags >> 4) & 3;
    let types = match (flags >> 8) & 3 {
        1 => Some(TypesMode::Jsdoc),
        2 => Some(TypesMode::Ts),
        _ => None,
    };
    let modern = match (flags >> 14) & 3 {
        0 => None,
        pick => untwist::parse_modern_baseline(BASELINES[pick as usize - 1]).ok(),
    };
    Options {
        unsafe_transforms: bit(flags, 0),
        svelte: bit(flags, 1),
        solid: bit(flags, 2),
        library_imports: bit(flags, 3),
        module_goal: goal == 1,
        script_goal: goal == 2,
        commonjs_goal: goal == 3,
        rename: RENAMES[((flags >> 10) & 7) as usize % RENAMES.len()],
        split_hoisted: bit(flags, 6),
        source_map: bit(flags, 7) || input_map.is_some(),
        single_file: bit(flags, 13),
        jsx: !bit(flags, 16),
        unpack: !bit(flags, 17),
        modern,
        types,
        input_source_maps: vec![input_map],
        max_iterations: Some(8),
        ..Options::default()
    }
}

pub fn run(data: &[u8]) {
    let Some((head, rest)) = data.split_first_chunk::<3>() else {
        return;
    };
    let flags = u32::from(head[0]) | u32::from(head[1]) << 8 | u32::from(head[2]) << 16;
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let (source, input_map) = match text.split_once(SOURCE_MAP_SEPARATOR) {
        Some((source, map)) => (source, Some(map.to_string())),
        None => (text, None),
    };
    let out = Pipeline::new(options_for(flags, input_map)).run(source);
    assert!(
        out.panicked.is_empty(),
        "the pipeline panicked (flags {flags:#08x}): {:?}",
        out.panicked
    );
    let dir = std::path::Path::new(PLAN_DIR);
    let (files, _refused) = out.plan_write(dir, &[]);
    for file in files {
        assert!(
            file.path.starts_with(dir) && untwist::is_safe_relative(&file.relative),
            "a planned file leaves the output directory: {}",
            file.path.display()
        );
    }
}
