pub mod build;
pub mod holder;
pub mod notes;
pub mod roles;
pub mod shell;

use oxc_allocator::{Allocator, CloneIn, Vec as AVec};
use oxc_ast::ast::{BindingPattern, Program, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_semantic::Scoping;
use oxc_span::{SourceType, SPAN};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::pass::{Note, NoteKind};
use untwist_core::semantic::build_scoping;

use crate::closed::captured_by_statement;
use crate::closure::build::assemble;
use crate::closure::holder::{holder_name, rewrite_shared, shared_vars};
use crate::closure::roles::{roles, Role, Stray, ENTRY_ID};
use crate::closure::shell::{probe_body, shell, Shell};
use crate::frame::Frame;
use crate::model::{body_key, Bodies, Bundle, BundleKind};
use crate::runtime::{
    declared_names, js_string, runtime_binding_import, runtime_id, synthetic_statements,
};
use crate::specifier::ModulePaths;

pub fn probe(frame: &Frame<'_, '_>) -> bool {
    shell(frame.top).is_some_and(|found| probe_body(&found))
}

fn shared_symbols(
    scoping: &Scoping,
    shell: &Shell<'_, '_>,
    statements: &[Statement<'_>],
    holder: &FxHashSet<String>,
) -> Option<FxHashSet<SymbolId>> {
    let mut symbols = FxHashSet::default();
    symbols.insert(shell.param.symbol_id.get()?);
    for statement in statements {
        if let Statement::VariableDeclaration(declaration) = statement {
            for declarator in &declaration.declarations {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                    symbols.insert(id.symbol_id.get()?);
                }
            }
        }
    }
    let wrapper = shell.function.scope_id.get()?;
    symbols.extend(
        scoping
            .get_bindings(wrapper)
            .iter()
            .filter(|(name, _)| holder.contains(name.as_str()))
            .map(|(_, &symbol)| symbol),
    );
    Some(symbols)
}

fn proven(
    scoping: &Scoping,
    shell: &Shell<'_, '_>,
    roles: &[Role],
    holder: &FxHashSet<String>,
) -> bool {
    let statements = &shell.body.statements;
    let Some(shared) = shared_symbols(scoping, shell, statements, holder) else {
        return false;
    };
    let written = shared.iter().any(|&symbol| {
        !holder.contains(scoping.symbol_name(symbol))
            && scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
    });
    !written
        && statements
            .iter()
            .zip(roles)
            .filter(|(_, role)| **role != Role::Prelude)
            .all(|(statement, _)| {
                let mut stray = Stray::new(scoping);
                oxc_ast_visit::VisitJs::visit_statement(&mut stray, statement);
                !stray.found && captured_by_statement(scoping, statement, &shared).is_none()
            })
}

fn excluded_names(
    shell: &Shell<'_, '_>,
    statements: &[Statement<'_>],
    roles: &[Role],
) -> Vec<String> {
    let mut names = vec![shell.param.name.to_string()];
    for (statement, role) in statements.iter().zip(roles) {
        if *role == Role::Prelude {
            names.extend(declared_names(std::slice::from_ref(statement)));
        }
    }
    names
}

fn planned_holder(
    source: &Program<'_>,
    shell: &Shell<'_, '_>,
    roles: &[Role],
    excluded: &[&str],
) -> Option<FxHashSet<String>> {
    let scoping = build_scoping(source).scoping;
    let wrapper = shell.function.scope_id.get()?;
    let holder = shared_vars(&scoping, wrapper, (&shell.body.statements, roles), excluded)?;
    proven(&scoping, shell, roles, &holder).then_some(holder)
}

type Owned<'a> = AVec<'a, Statement<'a>>;

fn owned_holder<'a>(
    alloc: &'a Allocator,
    (source, shell): (&Program<'a>, &Shell<'_, 'a>),
    owned: Owned<'a>,
    (roles, excluded): (&[Role], &[&str]),
) -> Result<(Owned<'a>, FxHashSet<String>), Owned<'a>> {
    let builder = AstBuilder::new(alloc);
    let mut program = Program::new(
        SPAN,
        SourceType::cjs(),
        source.source_text,
        AVec::new_in(&builder),
        None,
        shell.body.directives.clone_in(alloc),
        owned,
        &builder,
    );
    let scoping = build_scoping(&program).scoping;
    let root = scoping.root_scope_id();
    let Some(holder) = shared_vars(&scoping, root, (&program.body, roles), excluded) else {
        return Err(program.body);
    };
    if !holder.is_empty() {
        let name = holder_name(source.source_text);
        rewrite_shared(alloc, &scoping, &mut program.body, (&holder, &name));
    }
    Ok((program.body, holder))
}

pub fn detect_with<'k, 'a>(
    alloc: &'a Allocator,
    source: &'k Program<'a>,
    frame: &Frame<'k, 'a>,
    bodies: &mut Bodies<'a>,
) -> Option<Bundle<'a>> {
    let found = shell(frame.top)?;
    let taken = matches!(bodies, Bodies::Taken(_));
    let owned = bodies.statements(alloc, found.body);
    let list: &[Statement<'a>] = if taken {
        &owned
    } else {
        &found.body.statements
    };
    let plan = roles(source, &found.param.name, list)?;
    let excluded = excluded_names(&found, list, &plan);
    let excluded: Vec<&str> = excluded.iter().map(String::as_str).collect();
    let planned = match taken {
        true => None,
        false => Some(planned_holder(source, &found, &plan, &excluded)?),
    };
    let (owned, holder) = match planned {
        Some(holder) if owned.is_empty() => (owned, holder),
        planned => match owned_holder(alloc, (source, &found), owned, (&plan, &excluded)) {
            Ok((owned, holder)) if planned.as_ref().is_none_or(|p| *p == holder) => (owned, holder),
            Ok((owned, _)) | Err(owned) => {
                if let Bodies::Taken(pool) = bodies {
                    pool.insert(body_key(found.body), owned);
                }
                return None;
            }
        },
    };
    let name = (!holder.is_empty()).then(|| holder_name(source.source_text));
    assemble(alloc, source, &found, (&plan, name.is_some()), owned)
}

pub fn transforms<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> Vec<Note> {
    let Some(entry) = bundle.entry().map(|module| module.path.clone()) else {
        return Vec::new();
    };
    let runtime = runtime_id(BundleKind::Closure);
    let paths = ModulePaths::of(bundle);
    let text: String = bundle
        .modules
        .values()
        .filter(|module| !module.is_entry && module.id != runtime)
        .map(|module| {
            let target = paths.specifier(entry.as_str(), module.path.as_str());
            format!("require({});\n", js_string(&target))
        })
        .collect();
    let Some(statements) = synthetic_statements(alloc, text) else {
        return vec![Note {
            pass: "unpack",
            kind: NoteKind::Refusal,
            message: notes::requires_unbuilt(),
            span: None,
        }];
    };
    if let Some(module) = bundle.modules.get_mut(ENTRY_ID) {
        module.program_mut().body.extend(statements);
    }
    if bundle.modules.contains_key(&runtime) {
        hold(alloc, bundle, &runtime);
    }
    Vec::new()
}

fn hold<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>, runtime: &str) {
    for module in bundle
        .modules
        .values_mut()
        .filter(|module| module.id != runtime)
    {
        let name = holder_name(module.program().source_text);
        if let Some(import) =
            runtime_binding_import(alloc, &module.path, BundleKind::Closure, &name)
        {
            let rest = std::mem::replace(&mut module.program_mut().body, AVec::new_in(&alloc));
            let body = &mut module.program_mut().body;
            body.push(import);
            body.extend(rest);
        }
    }
}
