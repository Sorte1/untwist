use oxc_ast::ast::*;
use untwist_core::ast_util::readonly::UsesScan;

use super::Member;

impl<'a> Member<'a> for UsesScan<'_> {
    const PRUNES: bool = true;
    type Saved = ();

    #[inline(always)]
    fn enter_expression(&mut self, it: &Expression<'a>) -> Option<()> {
        self.descends_into(it).then_some(())
    }

    #[inline(always)]
    fn identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.reference(it);
    }
}
