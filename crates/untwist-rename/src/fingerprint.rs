use oxc_allocator::{Allocator, Box as ArenaBox, TakeIn};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::rename::Renames;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;

const PLACEHOLDER_STEM: &str = "$s";

pub fn alpha_fingerprint(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    if has_error(&parsed.diagnostics) {
        return None;
    }
    let sem = build_scoping(&parsed.program);
    let mut expander = ExpandShorthand {
        builder: AstBuilder::new(&alloc),
        keys: FxHashSet::default(),
    };
    expander.visit_program(&mut parsed.program);
    let prefix = free_prefix(&sem.scoping, &expander.keys);
    let mut order: Vec<SymbolId> = sem.scoping.symbol_ids().collect();
    order.sort_by_key(|&symbol| sem.scoping.symbol_span(symbol).start);
    let mut renames = Renames::new();
    for (index, symbol) in order.into_iter().enumerate() {
        let placeholder = format!("{prefix}{index}");
        renames.insert(symbol, Ident::from_str_in(&placeholder, &&alloc));
    }
    renames.apply(&mut parsed.program, &sem.scoping);
    Some(print(&parsed.program, PrintStyle::Compact))
}

fn free_prefix(scoping: &Scoping, keys: &FxHashSet<String>) -> String {
    let mut prefix = PLACEHOLDER_STEM.to_string();
    while scoping
        .root_unresolved_references()
        .keys()
        .any(|name| name.as_str().starts_with(&prefix))
        || keys.iter().any(|key| key.starts_with(&prefix))
    {
        prefix.push('s');
    }
    prefix
}

struct ExpandShorthand<'a> {
    builder: AstBuilder<'a>,
    keys: FxHashSet<String>,
}

impl<'a> ExpandShorthand<'a> {
    fn record(&mut self, key: &PropertyKey<'a>) {
        if let PropertyKey::StaticIdentifier(name) = key {
            self.keys.insert(name.name.as_str().to_string());
        }
    }

    fn expand_target(&mut self, property: &mut AssignmentTargetProperty<'a>) {
        let AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(taken) =
            property.take_in(&self.builder)
        else {
            return;
        };
        let taken = taken.unbox();
        let name = taken.binding.name;
        let span = taken.span;
        self.keys.insert(name.as_str().to_string());
        let key = PropertyKey::StaticIdentifier(ArenaBox::new_in(
            IdentifierName::new(taken.binding.span, name, &self.builder),
            &self.builder,
        ));
        let target = AssignmentTarget::AssignmentTargetIdentifier(ArenaBox::new_in(
            taken.binding,
            &self.builder,
        ));
        let binding = match taken.init {
            Some(init) => {
                AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(ArenaBox::new_in(
                    AssignmentTargetWithDefault::new(span, target, init, &self.builder),
                    &self.builder,
                ))
            }
            None => AssignmentTargetMaybeDefault::from(target),
        };
        *property = AssignmentTargetProperty::new_assignment_target_property_property(
            span,
            key,
            binding,
            false,
            &self.builder,
        );
    }
}

impl<'a> VisitMut<'a> for ExpandShorthand<'a> {
    fn visit_object_property(&mut self, it: &mut ObjectProperty<'a>) {
        it.shorthand = false;
        self.record(&it.key);
        walk_mut::walk_object_property(self, it);
    }

    fn visit_binding_property(&mut self, it: &mut BindingProperty<'a>) {
        it.shorthand = false;
        self.record(&it.key);
        walk_mut::walk_binding_property(self, it);
    }

    fn visit_assignment_target_property(&mut self, it: &mut AssignmentTargetProperty<'a>) {
        if matches!(
            it,
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(_)
        ) {
            self.expand_target(it);
        } else if let AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) = it {
            self.record(&property.name);
        }
        walk_mut::walk_assignment_target_property(self, it);
    }
}
