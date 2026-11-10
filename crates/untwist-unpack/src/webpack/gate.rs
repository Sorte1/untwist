use oxc_ast::ast::*;

use crate::matcher::param_names;
use crate::model::{BundleKind, Runtime};
use crate::notes::require_not_shared;
use crate::path::PosixPath;
use crate::runtime::runtime_path;
use crate::webpack::container::module_factories;
use crate::webpack::runtime_module::{uses_in, Needs};
use crate::UnpackOptions;

const BIND: &str = "bind";
const RUNTIME_NAME: &str = "webpack";
const RUNTIME_FILE: &str = "webpack.js";

pub struct Prelude<'r, 'a> {
    pub runtime: Option<&'r Runtime<'a>>,
}

fn claims_runtime(id: &str) -> bool {
    let named = id.ends_with(RUNTIME_NAME) || id.ends_with(RUNTIME_FILE);
    named
        && PosixPath::from_module_id(id, false)
            .is_some_and(|path| path.as_str() == runtime_path(BundleKind::Webpack))
}

fn shared_use(
    id: &str,
    require: &str,
    factory: &crate::matcher::Factory<'_, '_>,
    needs: Option<&Needs>,
) -> Result<(), String> {
    let uses = uses_in(require, &factory.body.statements);
    if uses.bare {
        return Err(require_not_shared(id, "passes it on or aliases it"));
    }
    if uses.computed {
        return Err(require_not_shared(id, "reaches it through a computed key"));
    }
    let routes = |name: &str| needs.is_some_and(|needs| needs.routes(name));
    let mut names: Vec<&String> = uses
        .names
        .iter()
        .filter(|name| name.as_str() != BIND && !routes(name))
        .collect();
    names.sort_unstable();
    match names.first() {
        Some(name) if needs.is_some() => Err(require_not_shared(
            id,
            format!("require.{name} is not a helper this bundle's runtime defines and moves"),
        )),
        Some(name) => Err(require_not_shared(
            id,
            format!("require.{name} has no runtime in this input"),
        )),
        None => Ok(()),
    }
}

pub fn routable(
    source: &str,
    container: &Expression<'_>,
    prelude: &Prelude<'_, '_>,
    options: UnpackOptions,
) -> Result<(), String> {
    if options.conservative {
        return Ok(());
    }
    let needs = prelude.runtime.map(Needs::of);
    for (id, factory) in module_factories(container) {
        if claims_runtime(&id) {
            return Err(require_not_shared(&id, "its id names the runtime module"));
        }
        let params = param_names(factory.params);
        let Some(require) = params.get(2) else {
            continue;
        };
        let span = factory.body.span;
        let text = source
            .get(span.start as usize..span.end as usize)
            .unwrap_or(source);
        if !text.contains(require) && !text.contains("\\u") {
            continue;
        }
        shared_use(&id, require, &factory, needs.as_ref())?;
    }
    Ok(())
}
