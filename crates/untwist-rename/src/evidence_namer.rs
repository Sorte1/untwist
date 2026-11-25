use oxc_syntax::symbol::SymbolId;

use crate::base_name::{base_name, BaseNamer, Context};
use crate::collect::{BindingInfo, BindingRole, Collected, InitKind};
use crate::evidence::{title, Evidence, Passing, Returned, DESCRIPTOR};
use crate::generate::to_identifier;
use crate::human::looks_human_written;
use crate::smart::{has_strong_base, smart_renameable};
use crate::usage::Usage;
use crate::RenameConfig;

const FORWARD_DEPTH: usize = 4;

const GENERIC: [&str; 22] = [
    "v",
    "fn",
    "cb",
    "result",
    "obj",
    "arr",
    "str",
    "num",
    "bool",
    "cls",
    "re",
    "instance",
    "predicate",
    "callback",
    "Ctor",
    "e",
    "i",
    "j",
    "k",
    "get",
    "set",
    "require",
];

const INVOKERS: [&str; 4] = ["call", "apply", "bind", "then"];

pub struct EvidenceNamer<'c> {
    pub collected: &'c Collected,
    pub evidence: &'c Evidence,
    pub usage: &'c Usage,
    pub config: RenameConfig,
}

fn usable(name: &str) -> bool {
    !GENERIC.contains(&name)
        && !INVOKERS.contains(&name)
        && !name.ends_with(|ch: char| ch.is_ascii_digit())
        && looks_human_written(name)
}

impl EvidenceNamer<'_> {
    fn binding(&self, symbol: SymbolId) -> Option<&BindingInfo> {
        let index = *self.collected.by_symbol.get(&symbol)?;
        self.collected.bindings.get(index)
    }

    fn evidenced(&self, symbol: SymbolId) -> Option<String> {
        let name = to_identifier(self.evidence.name(symbol)?);
        (!name.is_empty() && name != "_").then_some(name)
    }

    fn kept_name(&self, info: &BindingInfo) -> Option<String> {
        let kept = !smart_renameable(info, self.collected, self.config);
        (kept && usable(&info.name)).then(|| info.name.clone())
    }

    fn getter_key(&self, target: SymbolId) -> Option<&str> {
        let getter = self.evidence.getter(target)?;
        let Some(through) = getter.through else {
            return Some(&getter.key);
        };
        let BindingRole::Variable(shape) = &self.binding(through)?.role else {
            return None;
        };
        let reads = shape.kind == InitKind::Member && shape.name.as_deref() == Some(DESCRIPTOR);
        reads.then_some(getter.key.as_str())
    }

    fn traced(&self, symbol: SymbolId, depth: usize) -> Option<String> {
        if depth == 0 || self.usage.class_hint(symbol).is_some() {
            return None;
        }
        match self.evidence.returned(symbol)? {
            Returned::Forward(target, passing) => {
                let name = self.forwarded(*target, depth - 1)?;
                match passing {
                    Passing::Through => Some(name),
                    Passing::Keyed(key) => Some(format!("{name}{}", title(key))),
                    Passing::Computed => None,
                }
            }
            Returned::Local(local) if *local != symbol => self
                .local(*local)
                .map(|base| format!("get{}", title(&base))),
            Returned::Built(local) if *local != symbol => self
                .local(*local)
                .map(|base| format!("create{}", title(&base))),
            _ => None,
        }
    }

    fn forwarded(&self, target: SymbolId, depth: usize) -> Option<String> {
        if let Some(key) = self.getter_key(target) {
            return Some(format!("get{}", title(key)));
        }
        let info = self.binding(target)?;
        if has_strong_base(info, self.collected) {
            return None;
        }
        self.kept_name(info)
            .or_else(|| self.evidenced(target))
            .or_else(|| self.traced(target, depth))
            .filter(|name| usable(name))
    }

    fn local(&self, local: SymbolId) -> Option<String> {
        let info = self.binding(local)?;
        if let Some(kept) = self.kept_name(info) {
            return Some(kept);
        }
        if has_strong_base(info, self.collected) {
            return None;
        }
        let plain = Context {
            collected: self.collected,
            usage: self.usage,
            namer: None,
            config: self.config,
        };
        let base = self
            .evidenced(local)
            .unwrap_or_else(|| base_name(info, &plain));
        usable(&base).then_some(base)
    }
}

impl BaseNamer for EvidenceNamer<'_> {
    fn base_name(&self, binding: &BindingInfo, _usage: &Usage) -> Option<String> {
        if has_strong_base(binding, self.collected) {
            return None;
        }
        self.evidenced(binding.symbol)
            .or_else(|| self.traced(binding.symbol, FORWARD_DEPTH))
    }
}
