use oxc_span::Span;

use super::generate::{Gen, SiteKey};
use super::object::Obj;
use super::solve::{Field, Graph, Node, PatchKey};
use crate::mutants::builtin_aliases_trusted;
use crate::proof::aliased::{AliasKey, AliasSite};

const NAME_LIMIT: usize = 64;

fn targets_builtins(graph: &Graph, target: Node) -> Option<bool> {
    let mut any = false;
    let mut roots = false;
    for object in &graph.points_to(target) {
        match graph.objects.get(*object) {
            Obj::Builtin(builtin) if builtin.patchable() => {
                any = true;
                roots |= !builtin.is_method();
            }
            _ => {}
        }
    }
    any.then_some(!roots)
}

fn sorted_names(mut names: Vec<String>) -> AliasKey {
    names.sort();
    names.dedup();
    AliasKey::Names(names)
}

fn node_names(graph: &Graph, node: Node) -> AliasKey {
    let objects = graph.points_to(node);
    if objects.is_empty() || objects.len() > NAME_LIMIT {
        return AliasKey::Unknown;
    }
    let names: Option<Vec<String>> = objects
        .iter()
        .map(|object| graph.objects.as_str(*object).map(str::to_string))
        .collect();
    names.map_or(AliasKey::Unknown, sorted_names)
}

fn field_names(graph: &Graph, sources: &[Node]) -> AliasKey {
    let mut names = Vec::new();
    for source in sources {
        for object in &graph.points_to(*source) {
            if !graph.objects.is_site(*object) || graph.is_escaped(*object) {
                return AliasKey::Unknown;
            }
            for field in graph.named_fields(*object) {
                match field {
                    Field::ACCESSORS | Field::ENTRIES => {}
                    Field::UNKNOWN => return AliasKey::Unknown,
                    named => names.extend(graph.field_name(named).map(str::to_string)),
                }
            }
        }
    }
    sorted_names(names)
}

fn patch_key(graph: &Graph, key: &PatchKey) -> AliasKey {
    match key {
        PatchKey::Node(node) => node_names(graph, *node),
        PatchKey::Fields(sources) => field_names(graph, sources),
        PatchKey::Whole => AliasKey::Unknown,
        PatchKey::Absent => AliasKey::Names(vec!["undefined".to_string()]),
    }
}

impl Gen<'_> {
    pub(super) fn sites(&self) -> Vec<AliasSite> {
        if builtin_aliases_trusted() {
            return Vec::new();
        }
        if let Some(reason) = self.fallback {
            return vec![AliasSite::new(
                Span::default(),
                AliasKey::Fallback(reason),
                false,
            )];
        }
        if self.g.exhausted() {
            return vec![AliasSite::new(
                Span::default(),
                AliasKey::Fallback("its budget ran out"),
                false,
            )];
        }
        let graph = &self.g;
        let mut sites: Vec<(Span, AliasSite)> = Vec::new();
        for member in &self.members {
            let Some(functions_only) = targets_builtins(graph, member.base) else {
                continue;
            };
            let key = match &member.key {
                SiteKey::Name(name) => AliasKey::Names(vec![name.clone()]),
                SiteKey::Node(node) => node_names(graph, *node),
                SiteKey::Unknown => AliasKey::Unknown,
            };
            sites.push((
                member.span,
                AliasSite::new(member.span, key, functions_only),
            ));
        }
        for patch in &graph.patches {
            let Some(functions_only) = targets_builtins(graph, patch.target) else {
                continue;
            };
            let key = patch_key(graph, &patch.key);
            sites.push((patch.span, AliasSite::new(patch.span, key, functions_only)));
        }
        sites.sort_by_key(|(span, _)| (span.start, span.end));
        sites.into_iter().map(|(_, site)| site).collect()
    }

    pub(super) fn describe(&self, node: Node) -> Vec<String> {
        let mut found: Vec<String> = self
            .g
            .points_to(node)
            .iter()
            .map(|object| self.g.objects.describe(*object))
            .collect();
        found.sort();
        found
    }
}
