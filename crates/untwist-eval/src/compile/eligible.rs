use oxc_ast::ast::Statement;

use crate::decode::Checksum;

pub(crate) fn prelude<'a>(checksum: &Checksum<'a>) -> Option<&'a [Statement<'a>]> {
    checksum.body.get(..checksum.prelude.len())
}
