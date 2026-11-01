use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::keyword::{is_global_object, is_reserved_keyword};
use serde::Serialize;

use super::index::{Index, Kind, Occurrence, Shape, Symbol};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Edits(Vec<(u32, u32, String)>),
    Refused(String),
}

const RESTRICTED: [&str; 2] = ["arguments", "eval"];

fn check_name(name: &str) -> Result<(), String> {
    if !is_identifier_name(name) {
        return Err(format!("`{name}` is not an identifier"));
    }
    if is_reserved_keyword(name) || is_global_object(name) || RESTRICTED.contains(&name) {
        return Err(format!("`{name}` is a reserved word"));
    }
    Ok(())
}

fn check_symbol(index: &Index, occ: &Occurrence) -> Result<u32, String> {
    let name = index.names.get(occ.name);
    let Some(symbol) = index.symbol(occ.symbol) else {
        return Err(format!("`{name}` is not declared in this text"));
    };
    if symbol.exported {
        return Err(format!("`{name}` is exported"));
    }
    Ok(occ.symbol)
}

fn check_hazard(index: &Index, symbol: &Symbol) -> Result<(), String> {
    let Some(declaring) = index.scope(symbol.scope) else {
        return Ok(());
    };
    let subtree = index
        .scopes
        .get(symbol.scope as usize..=declaring.last_descendant as usize)
        .unwrap_or_default();
    if subtree.iter().any(|scope| scope.hazard) {
        return Err("a direct eval or with statement can see this binding".into());
    }
    Ok(())
}

fn check_shadowing(index: &Index, id: u32, symbol: &Symbol, name: u32) -> Result<(), String> {
    for occ in index.occurrences_of(id) {
        for scope in index.ancestors(occ.scope) {
            if index.binding(scope, name).is_some_and(|other| other != id) {
                return Err(format!(
                    "`{}` is already declared in a scope that sees this binding",
                    index.names.get(name)
                ));
            }
            if scope == symbol.scope {
                break;
            }
        }
    }
    Ok(())
}

fn check_capture(index: &Index, symbol: &Symbol, name: u32) -> Result<(), String> {
    let Some(declaring) = index.scope(symbol.scope) else {
        return Ok(());
    };
    let from = index
        .occurrences
        .partition_point(|occ| occ.start < declaring.start);
    let captured = index
        .occurrences
        .iter()
        .skip(from)
        .take_while(|occ| occ.start <= declaring.end)
        .filter(|occ| occ.name == name)
        .any(|occ| match index.symbol(occ.symbol) {
            Some(other) => !index.is_within(other.scope, symbol.scope),
            None => true,
        });
    if captured {
        return Err(format!(
            "an outer or global `{}` is used where this binding is visible",
            index.names.get(name)
        ));
    }
    Ok(())
}

fn check_jsx(index: &Index, id: u32, new: &str) -> Result<(), String> {
    let lowercase = new.starts_with(|c: char| c.is_ascii_lowercase());
    if lowercase
        && index
            .occurrences_of(id)
            .any(|occ| occ.shape == Shape::JsxTag)
    {
        return Err(format!(
            "`{new}` would turn its JSX tags into HTML elements"
        ));
    }
    Ok(())
}

fn is_sloppy_block(index: &Index, scope: u32) -> bool {
    index
        .scope(scope)
        .is_some_and(|scope| !scope.var_scope && !scope.strict)
}

fn check_annex_b(index: &Index, id: u32, symbol: &Symbol) -> Result<(), String> {
    let refused = Err("a sloppy-mode block function shares this name (Annex B)".to_string());
    if symbol.kind == Kind::Function && is_sloppy_block(index, symbol.scope) {
        return refused;
    }
    let Some(declaring) = index.scope(symbol.scope) else {
        return Ok(());
    };
    let shared = (symbol.scope..=declaring.last_descendant)
        .filter(|scope| is_sloppy_block(index, *scope))
        .filter_map(|scope| index.binding(scope, symbol.name))
        .any(|other| {
            other != id
                && index
                    .symbol(other)
                    .is_some_and(|s| s.kind == Kind::Function)
        });
    if shared {
        return refused;
    }
    Ok(())
}

fn edit(occ: &Occurrence, old: &str, new: &str) -> (u32, u32, String) {
    let text = match occ.shape {
        Shape::Plain | Shape::JsxTag => new.to_string(),
        Shape::Property => format!("{old}: {new}"),
        Shape::Import => format!("{old} as {new}"),
    };
    (occ.start, occ.end, text)
}

fn plan(index: &Index, pos: u32, new: &str) -> Result<Vec<(u32, u32, String)>, String> {
    if index
        .problems
        .iter()
        .any(|problem| problem.severity == "error")
    {
        return Err("the text has syntax errors; fix them first".into());
    }
    let occ = index
        .occurrence_at(pos)
        .ok_or_else(|| "no identifier here".to_string())?;
    let id = check_symbol(index, occ)?;
    check_name(new)?;
    let symbol = index.symbol(id).ok_or_else(|| "no symbol".to_string())?;
    let old = index.names.get(symbol.name);
    if old == new {
        return Ok(Vec::new());
    }
    check_hazard(index, symbol)?;
    check_annex_b(index, id, symbol)?;
    check_jsx(index, id, new)?;
    if let Some(name) = index.names.find(new) {
        check_shadowing(index, id, symbol, name)?;
        check_capture(index, symbol, name)?;
    }
    Ok(index
        .occurrences_of(id)
        .map(|occ| edit(occ, old, new))
        .collect())
}

pub fn rename(index: &Index, pos: u32, new: &str) -> Outcome {
    match plan(index, pos, new) {
        Ok(edits) => Outcome::Edits(edits),
        Err(reason) => Outcome::Refused(reason),
    }
}
