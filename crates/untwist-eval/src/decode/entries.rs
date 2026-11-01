use std::rc::Rc;

use oxc_ast::ast::Expression;
use rustc_hash::FxHashMap;

use super::checksum::{addition_terms, checksum_step, Checksum};
use super::{Call, Session, ROTATOR_THREW};
use crate::bail::{AbruptKind, Bail, EvalResult};
use crate::compile::entry::{compile_checksum, compile_in, eval_direct_tree, eval_leaf, is_direct};
use crate::compile::entry::{direct_checksum, direct_statement, direct_term};
use crate::compile::entry::{synthetic_frame, CompiledChecksum, TermSource};
use crate::compile::scope::ScopeChain;
use crate::compile::{LazyExpr, Reach};
use crate::env::declare;
use crate::frame::FrameRef;
use crate::value::{Abrupt, Eval, Value};

pub(crate) struct Prepared<'a> {
    pub(crate) checksum: Checksum<'a>,
    pub(crate) terms: Box<[&'a Expression<'a>]>,
    compiled: Option<Form<'a>>,
}

enum Form<'a> {
    Compiled(CompiledChecksum<'a>),
    Direct(Box<[TermSource<'a>]>),
}

#[derive(Default)]
pub(crate) struct Entries<'a> {
    root: Option<Rc<ScopeChain>>,
    rotator: Option<(usize, LazyExpr<'a>)>,
    arguments: FxHashMap<usize, LazyExpr<'a>>,
}

const UNLAID_ROOT: &str = "a session root the compiler could not lay out";
const UNLAID_CHECKSUM: &str = "a checksum the compiler could not lay out";
const PAST_THE_SUM: &str = "a checksum term past the sum";

fn address(expr: &Expression<'_>) -> usize {
    std::ptr::from_ref(expr) as usize
}

#[cfg(feature = "test-hooks")]
fn note(entry: crate::test_hooks::Entry) {
    crate::test_hooks::note_entry(entry);
}

impl<'a> Session<'a> {
    fn root_chain(&mut self) -> Result<Rc<ScopeChain>, Abrupt<'a>> {
        if self.entries.root.is_none() {
            self.entries.root = ScopeChain::root();
        }
        match &self.entries.root {
            Some(root) => Ok(root.clone()),
            None => Err(Bail::Unsupported(&UNLAID_ROOT).into()),
        }
    }

    pub(super) fn prepare(&mut self, checksum: Checksum<'a>) -> Prepared<'a> {
        #[cfg(feature = "test-hooks")]
        let _compiling = crate::test_hooks::Compiling::enter();
        let mut terms = Vec::new();
        addition_terms(checksum.expression, &mut terms);
        let compiled = match direct_checksum(&checksum, &terms) {
            true => Some(Form::Direct(
                terms
                    .iter()
                    .map(|term| TermSource::new(term, &mut self.interp.names, None))
                    .collect(),
            )),
            false => {
                compile_checksum(&checksum, &terms, &mut self.interp.compiler()).map(Form::Compiled)
            }
        };
        Prepared {
            checksum,
            terms: terms.into_boxed_slice(),
            compiled,
        }
    }

    pub(super) fn checksum_scope(
        &mut self,
        prepared: &Prepared<'a>,
    ) -> EvalResult<Option<(FrameRef<'a>, Value<'a>)>> {
        let opened = match &prepared.compiled {
            Some(Form::Direct(_)) => {
                #[cfg(feature = "test-hooks")]
                note(crate::test_hooks::Entry::ChecksumScope);
                self.direct_scope(&prepared.checksum)?
            }
            Some(Form::Compiled(compiled)) => {
                #[cfg(feature = "test-hooks")]
                note(crate::test_hooks::Entry::ChecksumScope);
                let scope = synthetic_frame(&self.interp, compiled.layout(), &self.env);
                checksum_step(compiled.open(&mut self.interp, scope))?
            }
            None => return Err(Bail::Unsupported(&UNLAID_CHECKSUM)),
        };
        #[cfg(feature = "test-hooks")]
        super::note_outcome(
            "scope",
            opened.as_ref().map(|(_, expected)| expected),
            self.interp.budget.steps_used(),
        );
        Ok(opened)
    }

    fn direct_scope(
        &mut self,
        checksum: &Checksum<'a>,
    ) -> EvalResult<Option<(FrameRef<'a>, Value<'a>)>> {
        let scope = self.interp.realm.new_env(Some(&self.env));
        let array = direct_term(&mut self.interp, &scope, checksum.array_argument);
        let Some(array_value) = checksum_step(array)? else {
            return Ok(None);
        };
        let array_param = self.interp.name_of(checksum.array_param);
        declare(&scope, array_param, array_value);
        let expected = direct_term(&mut self.interp, &scope, checksum.expected);
        let Some(expected) = checksum_step(expected)? else {
            return Ok(None);
        };
        let expected_param = self.interp.name_of(checksum.expected_param);
        declare(&scope, expected_param, expected.clone());
        for statement in &checksum.prelude {
            let hoisted = self.interp.hoist(std::slice::from_ref(*statement), &scope);
            if checksum_step(hoisted)?.is_none() {
                return Ok(None);
            }
        }
        for statement in &checksum.prelude {
            let ran = direct_statement(&mut self.interp, &scope, statement);
            if checksum_step(ran)?.is_none() {
                return Ok(None);
            }
        }
        Ok(Some((scope, expected)))
    }

    pub(super) fn eval_term(
        &mut self,
        prepared: &Prepared<'a>,
        index: usize,
        scope: &FrameRef<'a>,
    ) -> Eval<'a, Value<'a>> {
        if let Some(Form::Direct(sources)) = &prepared.compiled {
            if let Some(source) = sources.get(index) {
                #[cfg(feature = "test-hooks")]
                note(crate::test_hooks::Entry::ChecksumTerm);
                #[cfg(feature = "test-hooks")]
                note(crate::test_hooks::Entry::DirectTerm);
                return source.eval(&mut self.interp, scope);
            }
        }
        let compiled = match &prepared.compiled {
            Some(Form::Compiled(compiled)) => compiled.term(index),
            _ => None,
        };
        if let Some(term) = compiled {
            #[cfg(feature = "test-hooks")]
            note(crate::test_hooks::Entry::ChecksumTerm);
            #[cfg(feature = "test-hooks")]
            if term.is_direct() {
                note(crate::test_hooks::Entry::DirectTerm);
            }
            return term.eval(&mut self.interp, scope);
        }
        Err(Bail::Unsupported(&PAST_THE_SUM).into())
    }

    pub fn run_rotator(&mut self, iife: &'a Expression<'a>) -> EvalResult<()> {
        let scope = self.env.clone();
        match self.eval_at_root(iife, &scope) {
            Ok(_) => Ok(()),
            Err(abrupt) => match abrupt.into_kind() {
                AbruptKind::Bail(bail) => Err(bail),
                AbruptKind::Throw(_) => Err(Bail::Unsupported(&ROTATOR_THREW)),
            },
        }
    }

    fn eval_at_root(
        &mut self,
        iife: &'a Expression<'a>,
        scope: &FrameRef<'a>,
    ) -> Eval<'a, Value<'a>> {
        let key = address(iife);
        if self
            .entries
            .rotator
            .as_ref()
            .is_none_or(|(at, _)| *at != key)
        {
            let root = self.root_chain()?;
            self.entries.rotator = Some((key, compile_in(iife, &root)));
        }
        #[cfg(feature = "test-hooks")]
        note(crate::test_hooks::Entry::Rotator);
        match &self.entries.rotator {
            Some((_, code)) => code.eval(&mut self.interp, scope, Reach::CHECKED),
            None => Err(Bail::Unsupported(&UNLAID_ROOT).into()),
        }
    }

    pub(super) fn eval_call_arguments(&mut self, call: &Call<'a>) -> EvalResult<Vec<Value<'a>>> {
        let mut args = Vec::with_capacity(call.arguments.len());
        for argument in &call.arguments {
            match self.eval_argument(argument) {
                Ok(value) => args.push(value),
                Err(abrupt) => match abrupt.into_kind() {
                    AbruptKind::Bail(bail) => return Err(bail),
                    AbruptKind::Throw(_) => {
                        return Err(Bail::Unsupported(&"a decoder call's arguments threw"))
                    }
                },
            }
        }
        Ok(args)
    }

    fn eval_argument(&mut self, argument: &'a Expression<'a>) -> Eval<'a, Value<'a>> {
        if let Some(value) = eval_leaf(&mut self.interp, argument) {
            #[cfg(feature = "test-hooks")]
            note(crate::test_hooks::Entry::DirectArgument);
            return value;
        }
        let key = address(argument);
        if !self.entries.arguments.is_empty() {
            if let Some(code) = self.entries.arguments.get(&key) {
                #[cfg(feature = "test-hooks")]
                note(crate::test_hooks::Entry::CompiledArgument);
                return code.eval(&mut self.interp, &self.env, Reach::CHECKED);
            }
        }
        if is_direct(argument) {
            #[cfg(feature = "test-hooks")]
            note(crate::test_hooks::Entry::DirectArgument);
            return eval_direct_tree(&mut self.interp, argument);
        }
        self.first_argument(argument, key)
    }

    fn first_argument(&mut self, argument: &'a Expression<'a>, key: usize) -> Eval<'a, Value<'a>> {
        let root = self.root_chain()?;
        #[cfg(feature = "test-hooks")]
        note(crate::test_hooks::Entry::CachedArgument);
        let code = compile_in(argument, &root);
        #[cfg(feature = "test-hooks")]
        note(crate::test_hooks::Entry::CompiledArgument);
        let value = code.eval(&mut self.interp, &self.env, Reach::CHECKED);
        self.entries.arguments.insert(key, code);
        value
    }
}
