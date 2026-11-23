use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::base_name::FALLBACK_BASE;
use crate::collect::{BindingInfo, Collected};
use crate::evidence::Evidence;
use crate::human::looks_human_written;
use crate::smart::is_hex_token;
use crate::usage::Usage;

const ROUNDS: usize = 3;

struct Flow<'c> {
    collected: &'c Collected,
    evidence: &'c Evidence,
    usage: &'c Usage,
    bases: FxHashMap<SymbolId, String>,
}

impl Flow<'_> {
    fn target_name(&self, param: SymbolId) -> Option<String> {
        if let Some(base) = self.bases.get(&param) {
            return Some(base.clone());
        }
        let binding = self
            .collected
            .bindings
            .get(*self.collected.by_symbol.get(&param)?)?;
        looks_human_written(&binding.name).then(|| binding.name.clone())
    }

    fn agreed(&self, symbol: SymbolId) -> Option<String> {
        let sites = self.usage.passed.get(&symbol)?;
        let mut found: Option<String> = None;
        for (callee, index) in sites {
            let param = self.evidence.param(*callee, *index)?;
            if param == symbol {
                return None;
            }
            let name = self.target_name(param)?;
            match &found {
                Some(held) if *held != name => return None,
                _ => found = Some(name),
            }
        }
        found.filter(|name| name != FALLBACK_BASE && !is_hex_token(name))
    }
}

pub fn flow_arguments(
    planned: &mut [(&BindingInfo, String)],
    (collected, evidence, usage): (&Collected, &Evidence, &Usage),
) {
    let bases = planned
        .iter()
        .map(|(info, base)| (info.symbol, base.clone()))
        .collect();
    let mut flow = Flow {
        collected,
        evidence,
        usage,
        bases,
    };
    for _ in 0..ROUNDS {
        let mut changed = false;
        for (info, base) in planned.iter_mut() {
            if base != FALLBACK_BASE {
                continue;
            }
            if let Some(next) = flow.agreed(info.symbol) {
                flow.bases.insert(info.symbol, next.clone());
                *base = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}
