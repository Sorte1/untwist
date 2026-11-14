use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use untwist_core::ast_util::key_names::key_names;
use untwist_core::pass::PassOptions;
use untwist_core::semantic::pins::script_global_symbols;
use untwist_core::semantic::{
    has_module_syntax, hoist_blockers, is_shared_global, top_level_names_fixed, AnnexBWrites,
};

use crate::collect::{collect, Collected};
use crate::function_names::function_names;
use crate::oracle::OuterRule;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Setting {
    pub keep_globals: bool,
    pub keep_lexicals: bool,
    pub safe_lane: bool,
    pub classic_script: bool,
    pub extended: bool,
    pub library_names: bool,
    pub shared_counter: bool,
}

impl Setting {
    pub fn of(program: &Program<'_>, opts: &PassOptions) -> Self {
        Self {
            keep_globals: opts.keep_script_globals,
            keep_lexicals: opts.keep_script_lexicals,
            safe_lane: !opts.unsafe_transforms,
            classic_script: !opts.nested_root && !has_module_syntax(program),
            extended: opts.extended,
            library_names: !opts.rule_disabled(crate::library::rename::NAME),
            shared_counter: false,
        }
    }
}

pub fn prepare(
    program: &Program<'_>,
    scoping: &Scoping,
    rule: OuterRule,
    setting: Setting,
) -> Collected {
    let mut collected = collect(program, scoping, rule);
    collected.classic_script = setting.classic_script;
    if setting.classic_script && setting.extended {
        collected.global_keys = key_names(program, scoping);
    }
    pin_script_globals(&mut collected, scoping, setting);
    pin_annex_b_targets(&mut collected, program, scoping);
    if setting.safe_lane {
        pin_function_names(&mut collected, program, scoping);
    }
    if setting.library_names {
        pin_library_names(&mut collected, program, scoping);
    }
    collected
}

fn pin_library_names(collected: &mut Collected, program: &Program<'_>, scoping: &Scoping) {
    collected.library = crate::library::match_program(program, scoping);
    collected.pinned.extend(
        collected
            .library
            .names
            .iter()
            .filter(|(symbol, name)| scoping.symbol_name(**symbol) == name.as_str())
            .map(|(symbol, _)| *symbol),
    );
}

fn pin_script_globals(collected: &mut Collected, scoping: &Scoping, setting: Setting) {
    pin_shared_globals(collected, scoping);
    let keep_every_root = setting.classic_script && top_level_names_fixed(setting.shared_counter);
    if !setting.keep_globals && !setting.keep_lexicals && !keep_every_root {
        return;
    }
    if setting.keep_lexicals || keep_every_root {
        collected.pinned.extend(
            scoping
                .get_bindings(scoping.root_scope_id())
                .values()
                .copied(),
        );
    } else {
        collected.pinned.extend(script_global_symbols(scoping));
    }
}

fn pin_shared_globals(collected: &mut Collected, scoping: &Scoping) {
    collected.pinned.extend(
        scoping
            .get_bindings(scoping.root_scope_id())
            .iter()
            .filter(|(name, _)| is_shared_global(name.as_str()))
            .map(|(_, symbol)| *symbol),
    );
}

fn pin_annex_b_targets(collected: &mut Collected, program: &Program<'_>, scoping: &Scoping) {
    if collected.block_function_names.is_empty() {
        return;
    }
    let writes = AnnexBWrites::of(program, scoping);
    for function in writes.functions() {
        collected.pinned.insert(function.function);
        collected.pinned.extend(function.target);
    }
    collected.pinned.extend(hoist_blockers(scoping));
}

fn pin_function_names(collected: &mut Collected, program: &Program<'_>, scoping: &Scoping) {
    let names = function_names(program, scoping);
    if names.name_read {
        for giver in names.givers {
            if collected.pinned.insert(giver) {
                collected.name_pinned.insert(giver);
            }
        }
    }
}
