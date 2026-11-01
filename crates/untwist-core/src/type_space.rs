use oxc_allocator::ArenaVec;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolFlags;

#[derive(Default)]
struct TypeSpaceScan {
    found: bool,
}

impl<'a> Visit<'a> for TypeSpaceScan {
    fn visit_ts_type(&mut self, _it: &TSType<'a>) {
        self.found = true;
    }

    fn visit_ts_type_annotation(&mut self, _it: &TSTypeAnnotation<'a>) {
        self.found = true;
    }

    fn visit_ts_type_parameter_declaration(&mut self, _it: &TSTypeParameterDeclaration<'a>) {
        self.found = true;
    }

    fn visit_ts_type_parameter_instantiation(&mut self, _it: &TSTypeParameterInstantiation<'a>) {
        self.found = true;
    }

    fn visit_ts_this_parameter(&mut self, _it: &TSThisParameter<'a>) {
        self.found = true;
    }

    fn visit_ts_class_implements_list(&mut self, it: &ArenaVec<'a, TSClassImplements<'a>>) {
        self.found |= !it.is_empty();
    }

    fn visit_ts_interface_declaration(&mut self, _it: &TSInterfaceDeclaration<'a>) {
        self.found = true;
    }

    fn visit_ts_type_alias_declaration(&mut self, _it: &TSTypeAliasDeclaration<'a>) {
        self.found = true;
    }

    fn visit_ts_index_signature(&mut self, _it: &TSIndexSignature<'a>) {
        self.found = true;
    }

    fn visit_ts_namespace_export_declaration(&mut self, _it: &TSNamespaceExportDeclaration<'a>) {
        self.found = true;
    }
}

pub fn holds_type_space(program: &Program<'_>) -> bool {
    let mut scan = TypeSpaceScan::default();
    scan.visit_program(program);
    scan.found
}

pub fn declares_an_interface(scoping: &Scoping) -> bool {
    scoping
        .symbol_ids()
        .fold(SymbolFlags::empty(), |all, symbol| {
            all | scoping.symbol_flags(symbol)
        })
        .contains(SymbolFlags::Interface)
}
