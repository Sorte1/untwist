use std::cell::RefCell;
use std::rc::Rc;

use oxc_ast::ast::{Function, IdentifierReference, ImportMeta, NewTarget, Program};
use oxc_ast_visit::VisitJs;
use oxc_syntax::scope::ScopeFlags;

const ARGUMENTS: &str = "arguments";
const EVAL: &str = "eval";

thread_local! {
    static COMMONJS_TEXT: RefCell<Option<(usize, Rc<str>)>> = const { RefCell::new(None) };
}

pub struct CommonJsGuard {
    previous: Option<(usize, Rc<str>)>,
}

impl Drop for CommonJsGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        COMMONJS_TEXT.with(|cell| *cell.borrow_mut() = previous);
    }
}

pub fn assume_commonjs(text: Option<&str>) -> CommonJsGuard {
    let next = text.map(|text| (text.as_ptr() as usize, Rc::from(text)));
    let previous = COMMONJS_TEXT.with(|cell| cell.replace(next));
    CommonJsGuard { previous }
}

pub fn is_commonjs(program: &Program<'_>) -> bool {
    COMMONJS_TEXT.with(|cell| {
        cell.borrow().as_ref().is_some_and(|(at, text)| {
            let source = program.source_text;
            (*at == source.as_ptr() as usize && text.len() == source.len()) || **text == *source
        })
    })
}

pub fn top_level_is_local(program: &Program<'_>) -> bool {
    super::has_module_syntax(program) || is_commonjs(program)
}

#[derive(Default)]
struct WrapperReads {
    found: bool,
}

impl<'a> VisitJs<'a> for WrapperReads {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == ARGUMENTS || it.name == EVAL {
            self.found = true;
        }
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.found = true;
    }

    fn visit_import_meta(&mut self, _it: &ImportMeta) {
        self.found = true;
    }
}

pub fn reads_the_wrapper(program: &Program<'_>) -> bool {
    let mut scan = WrapperReads::default();
    scan.visit_program(program);
    scan.found
}
