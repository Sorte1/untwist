use untwist_core::pass::Pass;

pub mod base_name;
pub mod collect;
pub mod collisions;
pub mod fingerprint;
#[cfg(feature = "test-hooks")]
pub mod fresh;
pub mod generate;
mod hazards;
pub mod holders;
pub mod human;
pub mod import_path;
pub mod mangle;
pub mod oracle;
pub mod plan;
pub mod reserved;
pub mod shorthand;
pub mod simple;
pub mod unique;
pub mod usage;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenameMode {
    #[default]
    None,
    Mangle,
    Simple,
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
        RenameMode::None | RenameMode::Unique => Vec::new(),
    }
}

pub fn rename_passes<'a>(config: RenameConfig) -> Vec<Box<dyn Pass<'a> + 'a>> {
    if config.mode == RenameMode::Unique {
        let state = unique::shared_state();
        let Some(slot) = unique::reserve(&state, 0) else {
            return stateless_passes(config);
        };
        return vec![unique::boxed(config, state, slot)];
    }
    stateless_passes(config)
}

pub fn rename_passes_with_state<'a>(
    config: RenameConfig,
    state: &unique::SharedUniqueState,
    slot: unique::FileSlot,
) -> Vec<Box<dyn Pass<'a> + 'a>> {
    if config.mode == RenameMode::Unique {
        return vec![unique::boxed(config, std::sync::Arc::clone(state), slot)];
    }
    stateless_passes(config)
}
