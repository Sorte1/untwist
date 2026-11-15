use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::scope::ScopeFlags;

use crate::model::Bundle;
use crate::strict::written_as_module;

pub const IMPORT_META: &str = "import.meta";
pub const TOP_LEVEL_AWAIT: &str = "a top-level await";

#[derive(Default)]
struct ModuleOnly {
    functions: u32,
    found: Option<&'static str>,
}

impl ModuleOnly {
    fn at_top(&self) -> bool {
        self.functions == 0
    }

    fn awaits(&mut self) {
        if self.at_top() {
            self.found.get_or_insert(TOP_LEVEL_AWAIT);
        }
    }

    fn nested(&mut self, walk: impl FnOnce(&mut Self)) {
        self.functions += 1;
        walk(self);
        self.functions -= 1;
    }

    fn key(&mut self, key: &PropertyKey<'_>, computed: bool) {
        if computed {
            self.visit_property_key(key);
        }
    }
}

impl<'a> VisitJs<'a> for ModuleOnly {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.found.is_none() {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found.get_or_insert(IMPORT_META);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        self.awaits();
        walk_js::walk_await_expression(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if it.r#await {
            self.awaits();
        }
        walk_js::walk_for_of_statement(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind.is_await_using() {
            self.awaits();
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.nested(|this| walk_js::walk_function(this, it, flags));
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nested(|this| walk_js::walk_arrow_function_expression(this, it));
    }

    fn visit_class_element(&mut self, it: &ClassElement<'a>) {
        match it {
            ClassElement::MethodDefinition(method) => {
                self.visit_decorators(&method.decorators);
                self.key(&method.key, method.computed);
                self.visit_function(&method.value, ScopeFlags::Function);
            }
            ClassElement::PropertyDefinition(property) => {
                self.visit_decorators(&property.decorators);
                self.key(&property.key, property.computed);
                if let Some(value) = &property.value {
                    self.nested(|this| this.visit_expression(value));
                }
            }
            ClassElement::AccessorProperty(property) => {
                self.visit_decorators(&property.decorators);
                self.key(&property.key, property.computed);
                if let Some(value) = &property.value {
                    self.nested(|this| this.visit_expression(value));
                }
            }
            ClassElement::StaticBlock(block) => {
                self.nested(|this| walk_js::walk_static_block(this, block));
            }
            ClassElement::TSIndexSignature(_) => {}
        }
    }
}

pub fn module_only_syntax(program: &Program<'_>) -> Option<&'static str> {
    let mut scan = ModuleOnly::default();
    scan.visit_program(program);
    scan.found
}

fn runtime_module_syntax(bundle: &Bundle<'_>) -> Option<&'static str> {
    let runtime = bundle.runtime.as_ref()?;
    let mut scan = ModuleOnly::default();
    for helper in &runtime.helpers {
        scan.visit_statement(&helper.statement);
    }
    scan.found
}

pub fn written_module_syntax(bundle: &Bundle<'_>) -> Option<(String, &'static str)> {
    let module = bundle
        .modules
        .iter()
        .filter(|(_, module)| !written_as_module(module))
        .find_map(|(id, module)| {
            module_only_syntax(module.program()).map(|what| (format!("module {id}"), what))
        });
    module.or_else(|| runtime_module_syntax(bundle).map(|what| ("the runtime".to_string(), what)))
}
