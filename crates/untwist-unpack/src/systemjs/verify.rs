use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::semantic::build_scoping;

use super::exports::declarator_export;
use super::layout::{Bind, Declare, Layout};
use super::scan::{Params, Scan, Use, UseKind};

type Vars<'a> = (FxHashMap<SymbolId, Option<bool>>, Vec<&'a str>);

struct Facts<'a> {
    names: FxHashSet<&'a str>,
    ordered: bool,
    dynamic: Vec<&'a str>,
}

enum Init<'a> {
    None,
    Export(&'a str),
    Other,
}

enum Decl<'a> {
    Function,
    Class,
    Var(Init<'a>),
}

fn declare_vars<'a>(declare: &Declare<'_, 'a>) -> Option<Vars<'a>> {
    let params: Vec<&str> = declare.params.iter().map(|id| id.name.as_str()).collect();
    let mut roles: FxHashMap<&str, (Option<SymbolId>, Option<bool>)> = FxHashMap::default();
    for var in &declare.vars {
        let name = var.name.as_str();
        if params.contains(&name) || roles.insert(name, (var.symbol_id.get(), None)).is_some() {
            return None;
        }
    }
    let mut namespaces = Vec::new();
    for bind in declare.setters.iter().flatten() {
        let (local, namespace) = match bind {
            Bind::Import(_, local) => (local, false),
            Bind::Namespace(local) => (local, true),
            Bind::ReExport(..) => continue,
        };
        let role = roles.get_mut(local.name.as_str())?;
        if role.1.replace(namespace).is_some() {
            return None;
        }
        if namespace {
            namespaces.push(local.name.as_str());
        }
    }
    let vars = roles
        .into_values()
        .map(|(symbol, role)| Some((symbol?, role)))
        .collect::<Option<_>>()?;
    Some((vars, namespaces))
}

fn execute_names_free(scoping: &Scoping, declare: &Declare<'_, '_>) -> bool {
    let Some(scope) = declare.execute_scope.get() else {
        return false;
    };
    let declared = declare.params.iter().chain(&declare.vars);
    declared
        .into_iter()
        .all(|id| !scoping.scope_has_binding(scope, id.name.as_str().into()))
}

fn declaration<'a>(
    body: &FunctionBody<'a>,
    span: Span,
    export: Option<&str>,
) -> Option<(usize, Decl<'a>)> {
    let statements = &body.statements;
    let index = statements.partition_point(|s| s.span().end <= span.start);
    let decl = match statements.get(index)? {
        Statement::FunctionDeclaration(f) if f.id.as_ref().map(|id| id.span) == Some(span) => {
            Decl::Function
        }
        Statement::ClassDeclaration(c) if c.id.as_ref().map(|id| id.span) == Some(span) => {
            Decl::Class
        }
        Statement::VariableDeclaration(declaration) => {
            let declarator = declaration
                .declarations
                .iter()
                .find(|d| d.id.span() == span)?;
            Decl::Var(
                match (&declarator.init, declarator_export(declarator, export)) {
                    (_, Some((_, name))) => Init::Export(name),
                    (None, None) => Init::None,
                    (Some(_), None) => Init::Other,
                },
            )
        }
        _ => return None,
    };
    Some((index, decl))
}

fn decl_ok(name: &str, (index, decl): (Option<usize>, Decl<'_>), reads: &[Option<usize>]) -> bool {
    if reads.is_empty() {
        return match decl {
            Decl::Var(Init::None) => true,
            Decl::Var(Init::Export(declared)) => declared == name,
            _ => false,
        };
    }
    let after = |at: usize| reads.iter().any(|top| top.is_some_and(|top| top > at));
    match (decl, index) {
        (Decl::Function, Some(_)) => reads.iter().any(Option::is_some),
        (Decl::Class | Decl::Var(Init::Export(_) | Init::Other), Some(at)) => after(at),
        _ => false,
    }
}

struct Group<'u, 'a> {
    name: &'a str,
    uses: Vec<&'u Use<'a>>,
}

fn group_symbol(group: &Group<'_, '_>) -> Option<(SymbolId, usize, Vec<Option<usize>>)> {
    let (mut symbol, mut wrapped, mut reads) = (None, 0, Vec::new());
    for found in &group.uses {
        let current = match found.kind {
            UseKind::Fresh => return None,
            UseKind::Write(s) => {
                wrapped += 1;
                s
            }
            UseKind::Read(s) => {
                reads.push(found.top);
                s
            }
            UseKind::Declared(s) => s,
        }?;
        if symbol.is_some_and(|seen| seen != current) {
            return None;
        }
        symbol = Some(current);
    }
    Some((symbol?, wrapped, reads))
}

fn group_ok(
    scoping: &Scoping,
    declare: &Declare<'_, '_>,
    vars: &Vars<'_>,
    group: &Group<'_, '_>,
) -> bool {
    if group.uses.iter().all(|u| matches!(u.kind, UseKind::Fresh)) {
        return true;
    }
    let Some((symbol, wrapped, reads)) = group_symbol(group) else {
        return false;
    };
    let writes = scoping
        .get_resolved_references(symbol)
        .filter(|r| r.is_write())
        .count();
    if !scoping.symbol_redeclarations(symbol).is_empty() || writes != wrapped {
        return false;
    }
    let decl = match vars.0.get(&symbol) {
        Some(None) => (None, Decl::Var(Init::None)),
        Some(Some(_)) => return false,
        None if Some(scoping.symbol_scope_id(symbol)) == declare.execute_scope.get() => {
            let span = scoping.symbol_span(symbol);
            match declaration(declare.execute, span, declare.export_name()) {
                Some((index, decl)) => (Some(index), decl),
                None => return false,
            }
        }
        None => return false,
    };
    decl_ok(group.name, decl, &reads)
}

fn groups<'u, 'a>(uses: &'u [Use<'a>]) -> Vec<Group<'u, 'a>> {
    let mut index: FxHashMap<&str, usize> = FxHashMap::default();
    let mut groups: Vec<Group<'u, 'a>> = Vec::new();
    for found in uses {
        let at = *index.entry(found.name).or_insert_with(|| {
            groups.push(Group {
                name: found.name,
                uses: Vec::new(),
            });
            groups.len() - 1
        });
        groups[at].uses.push(found);
    }
    groups
}

fn ordered(re_exports: &[&str], uses: &[Use<'_>]) -> bool {
    if uses.iter().any(|u| u.top.is_none()) {
        return false;
    }
    let mut seen = FxHashSet::default();
    let mut last: Option<&str> = None;
    for name in re_exports
        .iter()
        .copied()
        .chain(uses.iter().map(|u| u.name))
    {
        if !seen.insert(name) {
            continue;
        }
        if last.is_some_and(|previous| previous >= name) {
            return false;
        }
        last = Some(name);
    }
    true
}

fn re_export_names<'a>(declare: &Declare<'_, 'a>) -> Vec<&'a str> {
    declare
        .setters
        .iter()
        .flatten()
        .filter_map(|bind| match bind {
            Bind::ReExport(_, exported) => Some(*exported),
            _ => None,
        })
        .collect()
}

fn facts<'a>(
    scoping: &Scoping,
    declare: &Declare<'_, 'a>,
    (names, keep_names): (&FxHashSet<&'a str>, bool),
) -> Option<Facts<'a>> {
    let vars = declare_vars(declare)?;
    if !execute_names_free(scoping, declare) {
        return None;
    }
    let param = |at: usize| {
        declare
            .params
            .get(at)
            .map(|id| (id.name.as_str(), id.symbol_id.get()))
    };
    let params = Params {
        export: param(0),
        context: param(1),
    };
    let mut scan = Scan::new(
        scoping,
        params,
        (&vars.0, &vars.1),
        names,
        declare.execute.span,
    );
    scan.execute(declare.execute);
    let groups = groups(&scan.uses);
    if scan.refused || !groups.iter().all(|g| group_ok(scoping, declare, &vars, g)) {
        return None;
    }
    let re_exports = re_export_names(declare);
    let mut all: FxHashSet<&str> = FxHashSet::default();
    let unique = re_exports
        .iter()
        .all(|name| super::exports::is_export_name(name) && all.insert(name));
    if !unique || groups.iter().any(|g| !all.insert(g.name)) {
        return None;
    }
    if !keep_names {
        all = FxHashSet::default();
    }
    Some(Facts {
        ordered: ordered(&re_exports, &scan.uses),
        names: all,
        dynamic: scan.dynamic,
    })
}

fn linked(layout: &Layout<'_, '_>, facts: &FxHashMap<&str, Facts<'_>>) -> bool {
    let binds_ok = layout.live.iter().all(|&at| {
        let register = &layout.registers[at];
        register
            .deps
            .iter()
            .zip(&register.declare.setters)
            .all(|(dep, binds)| {
                let Some(target) = facts.get(dep) else {
                    return true;
                };
                binds.iter().all(|bind| match bind {
                    Bind::Import(name, _) | Bind::ReExport(name, _) => target.names.contains(name),
                    Bind::Namespace(_) => true,
                })
            })
    });
    let entry = layout.id(layout.entry);
    let mut observable = facts.values().flat_map(|f| f.dynamic.iter().copied());
    binds_ok
        && observable.all(|id| facts.get(id).is_some_and(|f| f.ordered))
        && facts.get(entry).is_some_and(|f| f.ordered)
}

fn named_targets<'a>(layout: &Layout<'_, 'a>) -> FxHashSet<&'a str> {
    let mut targets = FxHashSet::default();
    for register in &layout.registers {
        for (dep, binds) in register.deps.iter().zip(&register.declare.setters) {
            let named = binds.iter().any(|bind| !matches!(bind, Bind::Namespace(_)));
            if named {
                targets.insert(*dep);
            }
        }
    }
    targets
}

pub fn verify(source: &Program<'_>, layout: &Layout<'_, '_>) -> bool {
    let scoping = build_scoping(source).scoping;
    let names = layout.names();
    let named = named_targets(layout);
    let mut all = FxHashMap::default();
    for &at in &layout.live {
        let keep = named.contains(layout.id(at));
        let Some(found) = facts(&scoping, &layout.registers[at].declare, (&names, keep)) else {
            return false;
        };
        all.insert(layout.id(at), found);
    }
    linked(layout, &all)
}
