use untwist_core::pass::Pass;

pub mod base_name;
pub mod collect;
pub mod collisions;
pub mod commonjs;
pub mod evidence;
pub mod fingerprint;
#[cfg(feature = "test-hooks")]
pub mod fresh;
pub mod function_names;
pub mod generate;
mod hazards;
pub mod holders;
pub mod human;
pub mod import_path;
pub mod library;
pub mod mangle;
#[cfg(feature = "modern")]
pub mod modern;
pub mod oracle;
pub mod pins;
pub mod plan;
mod prototype_methods;
pub mod reserved;
pub mod shorthand;
pub mod simple;
pub mod smart;
pub mod text_reads;
pub mod unique;
pub mod usage;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenameMode {
    #[default]
    None,
    Mangle,
    Simple,
    Smart,
    Unique,
}

#[derive(Clone, Copy, Debug)]
pub struct RenameConfig {
    pub mode: RenameMode,
    pub rename_all: bool,
    pub extended: bool,
}

impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            mode: RenameMode::None,
            rename_all: true,
            extended: true,
        }
    }
}

fn stateless_passes<'a>(config: RenameConfig) -> Vec<Box<dyn Pass<'a> + 'a>> {
    match config.mode {
        RenameMode::Mangle => vec![mangle::boxed(config)],
        RenameMode::Simple => vec![simple::boxed(config)],
        RenameMode::Smart => vec![smart::boxed(config)],
        RenameMode::None | RenameMode::Unique => Vec::new(),
    }
}

fn with_library_names<'a>(
    config: RenameConfig,
    unique: Option<(unique::SharedUniqueState, usize)>,
    passes: Vec<Box<dyn Pass<'a> + 'a>>,
) -> Vec<Box<dyn Pass<'a> + 'a>> {
    if config.mode == RenameMode::None {
        return passes;
    }
    let mut all = vec![library::rename::boxed(config, unique)];
    all.extend(passes);
    all
}

pub fn rename_passes<'a>(config: RenameConfig) -> Vec<Box<dyn Pass<'a> + 'a>> {
    if config.mode == RenameMode::Unique {
        let state = unique::shared_state();
        if let Some(slot) = unique::reserve(&state, 0) {
            return rename_passes_with_state(config, &state, slot);
        }
    }
    with_library_names(config, None, stateless_passes(config))
}

pub fn rename_passes_with_state<'a>(
    config: RenameConfig,
    state: &unique::SharedUniqueState,
    slot: unique::FileSlot,
) -> Vec<Box<dyn Pass<'a> + 'a>> {
    if config.mode == RenameMode::Unique {
        let shared = Some((std::sync::Arc::clone(state), slot.index()));
        let unique = vec![unique::boxed(config, std::sync::Arc::clone(state), slot)];
        return with_library_names(config, shared, unique);
    }
    with_library_names(config, None, stateless_passes(config))
}
