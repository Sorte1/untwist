use std::fmt;

use thiserror::Error;

use crate::value::Value;

#[derive(Clone, PartialEq, Eq)]
pub struct GlobalName(pub String);

impl fmt::Display for GlobalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for GlobalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Bail {
    #[error("step budget exhausted")]
    Fuel,
    #[error("loop iteration cap exceeded")]
    LoopCap,
    #[error("call depth cap exceeded")]
    CallDepth,
    #[error("nesting depth cap exceeded")]
    NestingDepth,
    #[error("string length cap exceeded")]
    StringLength,
    #[error("array length cap exceeded")]
    ArrayLength,
    #[error("unsupported construct: {0}")]
    Unsupported(&'static &'static str),
    #[error("unresolved global binding: {0}")]
    UnresolvedGlobal(Box<GlobalName>),
    #[error("hostile global {0} reached")]
    HostileGlobal(Box<GlobalName>),
    #[error("{0} chain too deep")]
    ChainTooDeep(&'static &'static str),
    #[error("the input's heap budget is spent")]
    Budget,
    #[error("a property's attributes refuse it")]
    Refused,
    #[error("{}: the run reached its budget", .0.as_str())]
    RunBudget(untwist_core::budget::Hit),
}

impl Bail {
    pub fn unresolved_global(name: impl Into<String>) -> Self {
        Self::UnresolvedGlobal(Box::new(GlobalName(name.into())))
    }

    pub fn hostile_global(name: impl Into<String>) -> Self {
        Self::HostileGlobal(Box::new(GlobalName(name.into())))
    }
}

pub type EvalResult<T> = Result<T, Bail>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    TypeError,
    RangeError,
    UriError,
    SyntaxError,
    ReferenceError,
}

impl ErrorKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::TypeError => "TypeError",
            Self::RangeError => "RangeError",
            Self::UriError => "URIError",
            Self::SyntaxError => "SyntaxError",
            Self::ReferenceError => "ReferenceError",
        }
    }
}

#[derive(Clone, Debug)]
pub enum AbruptKind<'a> {
    Throw(Value<'a>),
    Bail(Bail),
}

#[derive(Clone)]
pub struct Abrupt<'a>(Box<AbruptKind<'a>>);

impl fmt::Debug for Abrupt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl<'a> Abrupt<'a> {
    pub fn throw(value: Value<'a>) -> Self {
        Self(Box::new(AbruptKind::Throw(value)))
    }

    pub fn kind(&self) -> &AbruptKind<'a> {
        &self.0
    }

    pub fn into_kind(self) -> AbruptKind<'a> {
        *self.0
    }

    pub fn is_bail(&self) -> bool {
        matches!(*self.0, AbruptKind::Bail(_))
    }

    pub fn into_thrown(self) -> Result<Value<'a>, Self> {
        if self.is_bail() {
            return Err(self);
        }
        match self.into_kind() {
            AbruptKind::Throw(value) => Ok(value),
            AbruptKind::Bail(bail) => Err(bail.into()),
        }
    }
}

impl From<Bail> for Abrupt<'_> {
    fn from(bail: Bail) -> Self {
        Self(Box::new(AbruptKind::Bail(bail)))
    }
}

pub type Eval<'a, T> = Result<T, Abrupt<'a>>;
