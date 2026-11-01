use crate::bail::Bail;
use crate::compile::stmt::statement as leveled_statement;
use crate::compile::{leveled, Code, Stmt};
use crate::value::Abrupt;

pub(crate) fn expression<'a>(reason: &'static &'static str) -> Code<'a> {
    leveled(move |_, _, _| Err(Bail::Unsupported(reason).into()))
}

pub(crate) fn unported<'a>() -> Code<'a> {
    expression(&"expression kind")
}

pub(crate) fn statement<'a>() -> Stmt<'a> {
    leveled_statement(|_, _, _| Err(Bail::Unsupported(&"statement kind").into()))
}

pub(crate) fn mismatch<'a>() -> Abrupt<'a> {
    Bail::Unsupported(&"a compiled assignment whose target lost its place").into()
}
