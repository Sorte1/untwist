use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashSet;

use super::baseline::Version;
use super::plan::LABEL;
use crate::library::shape::{exact_shape, shape_program, UnitKind};
use crate::library::table::{current, exact_key, loose_key, Role, Row, Table};

pub struct Lookalike {
    pub span: Span,
    pub modules: Vec<String>,
    pub versions: (String, String),
}

fn versions(table: &Table, rows: &[&Row]) -> Option<(String, String)> {
    let mut all: Vec<Version> = rows
        .iter()
        .flat_map(|row| {
            table
                .versions(row)
                .split(',')
                .flat_map(|range| range.split('~'))
        })
        .filter_map(Version::parse)
        .collect();
    all.sort();
    Some((all.first()?.to_string(), all.last()?.to_string()))
}

pub fn lookalikes(
    program: &Program<'_>,
    scoping: &Scoping,
    known: &FxHashSet<Span>,
) -> Vec<Lookalike> {
    let shared = current();
    let table: &Table = &shared;
    let shaped = shape_program(program, scoping);
    let mut found = Vec::new();
    for unit in &shaped.units {
        if unit.kind != UnitKind::Function || known.contains(&unit.span) {
            continue;
        }
        if !table.has_loose(loose_key(unit.loose_key(&shaped.prefix))) {
            continue;
        }
        let shape = exact_shape(unit, scoping);
        let rows: Vec<&Row> = table
            .rows_for(exact_key(shape.exact))
            .filter(|row| row.loose == loose_key(shape.loose))
            .collect();
        let wrappers = rows
            .iter()
            .all(|row| table.label(row) == LABEL && row.role == Role::Wrapper);
        if rows.is_empty() || !wrappers {
            continue;
        }
        let mut modules: Vec<String> = rows
            .iter()
            .map(|row| table.module(row).to_string())
            .collect();
        modules.sort();
        modules.dedup();
        if let Some(versions) = versions(table, &rows) {
            found.push(Lookalike {
                span: unit.span,
                modules,
                versions,
            });
        }
    }
    found
}
