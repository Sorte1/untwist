use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use untwist_core::pass::PassOptions;
use untwist_core::semantic::pins::script_global_symbols;
use untwist_core::semantic::{has_module_syntax, AnnexBWrites};

use crate::collect::{collect, Collected};
use crate::function_names::function_names;
use crate::oracle::OuterRule;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Setting {
    pub keep_globals: bool,
    pub keep_lexicals: bool,
    pub safe_lane: bool,
    pub classic_script: bool,
}

impl Setting {
    pub fn of(program: &Program<'_>, opts: &PassOptions) -> Self {
        Self {
            keep_globals: opts.keep_script_globals,
            keep_lexicals: opts.keep_script_lexicals,
            safe_lane: !opts.unsafe_transforms,
            classic_script: !opts.nested_root && !has_module_syntax(program),
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
    pin_script_globals(&mut collected, scoping, setting);
    pin_annex_b_targets(&mut collected, program, scoping);
    if setting.safe_lane {
        pin_function_names(&mut collected, program, scoping);
    }
    collected
}

fn pin_script_globals(collected: &mut Collected, scoping: &Scoping, setting: Setting) {
    if !setting.keep_globals && !setting.keep_lexicals {
        return;
    }
    if setting.keep_lexicals {
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

fn pin_annex_b_targets(collected: &mut Collected, program: &Program<'_>, scoping: &Scoping) {
    if collected.block_function_names.is_empty() {
        return;
    }
    let writes = AnnexBWrites::of(program, scoping);
    for function in writes.functions() {
        collected.pinned.insert(function.function);
        collected.pinned.extend(function.target);
    }
}

fn pin_function_names(collected: &mut Collected, program: &Program<'_>, scoping: &Scoping) {
    let names = function_names(program, scoping);
    if names.name_read {
        collected.pinned.extend(names.givers);
    }
}
