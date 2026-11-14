use std::cmp::Ordering;
use std::fmt::Write as _;

pub const MAX_LITERALS: usize = 6;
const MAX_STRING_LITERAL: usize = 40;
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Copy, Debug)]
pub struct Num(pub f64);

impl PartialEq for Num {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Num {}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Num {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Member {
    StrLit(String),
    NumLit(Num),
    True,
    False,
    Number,
    String,
    Boolean,
    BigInt,
    Promise(Box<Ty>),
    Null,
    Undefined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Number,
    String,
    Boolean,
    BigInt,
    Promise,
    Null,
    Undefined,
}

impl Member {
    pub fn kind(&self) -> Kind {
        match self {
            Member::StrLit(_) | Member::String => Kind::String,
            Member::NumLit(_) | Member::Number => Kind::Number,
            Member::True | Member::False | Member::Boolean => Kind::Boolean,
            Member::BigInt => Kind::BigInt,
            Member::Promise(_) => Kind::Promise,
            Member::Null => Kind::Null,
            Member::Undefined => Kind::Undefined,
        }
    }

    fn is_literal(&self) -> bool {
        matches!(
            self,
            Member::StrLit(_) | Member::NumLit(_) | Member::True | Member::False
        )
    }

    fn widened(&self) -> Member {
        match self {
            Member::StrLit(_) => Member::String,
            Member::NumLit(_) => Member::Number,
            Member::True | Member::False => Member::Boolean,
            Member::Promise(inner) => Member::Promise(Box::new(inner.widened())),
            other => other.clone(),
        }
    }

    fn covers(&self, other: &Member) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            (Member::String, Member::StrLit(_)) => true,
            (Member::Number, Member::NumLit(_)) => true,
            (Member::Boolean, Member::True | Member::False) => true,
            (Member::Promise(outer), Member::Promise(inner)) => outer.contains(inner),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ty {
    members: Vec<Member>,
}

impl Ty {
    pub fn of(member: Member) -> Ty {
        Ty::from_members(vec![member])
    }

    pub fn from_members(members: Vec<Member>) -> Ty {
        normalized(members)
    }

    pub fn number() -> Ty {
        Ty::of(Member::Number)
    }

    pub fn string() -> Ty {
        Ty::of(Member::String)
    }

    pub fn boolean() -> Ty {
        Ty::of(Member::Boolean)
    }

    pub fn bigint() -> Ty {
        Ty::of(Member::BigInt)
    }

    pub fn undefined() -> Ty {
        Ty::of(Member::Undefined)
    }

    pub fn null() -> Ty {
        Ty::of(Member::Null)
    }

    pub fn string_literal(text: &str) -> Ty {
        let printable = text.chars().count() <= MAX_STRING_LITERAL
            && text
                .chars()
                .all(|c| (' '..='~').contains(&c) && c != '"' && c != '\\');
        match printable {
            true => Ty::of(Member::StrLit(text.to_string())),
            false => Ty::string(),
        }
    }

    pub fn number_literal(value: f64) -> Ty {
        let integral = value.is_finite()
            && value.fract() == 0.0
            && value.abs() <= MAX_SAFE_INTEGER
            && !(value == 0.0 && value.is_sign_negative());
        match integral {
            true => Ty::of(Member::NumLit(Num(value))),
            false => Ty::number(),
        }
    }

    pub fn boolean_literal(value: bool) -> Ty {
        Ty::of(if value { Member::True } else { Member::False })
    }

    pub fn promise(inner: Ty) -> Ty {
        Ty::of(Member::Promise(Box::new(inner)))
    }

    pub fn members(&self) -> &[Member] {
        &self.members
    }

    pub fn join(&self, other: &Ty) -> Ty {
        let mut members = self.members.clone();
        members.extend(other.members.iter().cloned());
        normalized(members)
    }

    pub fn widened(&self) -> Ty {
        normalized(self.members.iter().map(Member::widened).collect())
    }

    pub fn contains(&self, other: &Ty) -> bool {
        other
            .members
            .iter()
            .all(|member| self.members.iter().any(|mine| mine.covers(member)))
    }

    pub fn has_literals(&self) -> bool {
        self.members.iter().any(Member::is_literal)
    }

    pub fn kinds(&self) -> impl Iterator<Item = Kind> + '_ {
        self.members.iter().map(Member::kind)
    }

    pub fn only(&self, kind: Kind) -> bool {
        !self.members.is_empty() && self.kinds().all(|k| k == kind)
    }

    pub fn any_of(&self, kind: Kind) -> bool {
        self.kinds().any(|k| k == kind)
    }

    pub fn single_kind(&self) -> Option<Kind> {
        let first = self.members.first()?.kind();
        self.only(first).then_some(first)
    }

    pub fn is_nullable(&self) -> bool {
        self.any_of(Kind::Null) || self.any_of(Kind::Undefined)
    }

    pub fn is_exactly_undefined(&self) -> bool {
        self.members == [Member::Undefined]
    }

    pub fn without_undefined(&self) -> Option<Ty> {
        let members: Vec<Member> = self
            .members
            .iter()
            .filter(|member| **member != Member::Undefined)
            .cloned()
            .collect();
        (!members.is_empty()).then(|| normalized(members))
    }

    pub fn overlaps(&self, other: &Ty) -> bool {
        self.members.iter().any(|mine| {
            other
                .members
                .iter()
                .any(|theirs| mine.covers(theirs) || theirs.covers(mine))
        })
    }

    pub fn promise_free(&self) -> bool {
        !self.any_of(Kind::Promise)
    }

    pub fn ts(&self) -> String {
        let mut text = String::new();
        for (index, member) in self.members.iter().enumerate() {
            if index > 0 {
                text.push_str(" | ");
            }
            write_member(&mut text, member);
        }
        text
    }

    pub fn predicate(&self, value: &str) -> String {
        let tests: Vec<String> = self
            .members
            .iter()
            .map(|member| member_predicate(member, value))
            .collect();
        format!("({})", tests.join(" || "))
    }
}

fn write_member(text: &mut String, member: &Member) {
    match member {
        Member::StrLit(value) => {
            let _ = write!(text, "\"{value}\"");
        }
        Member::NumLit(Num(value)) => {
            let _ = write!(text, "{}", *value as i64);
        }
        Member::True => text.push_str("true"),
        Member::False => text.push_str("false"),
        Member::Number => text.push_str("number"),
        Member::String => text.push_str("string"),
        Member::Boolean => text.push_str("boolean"),
        Member::BigInt => text.push_str("bigint"),
        Member::Promise(inner) => {
            let _ = write!(text, "Promise<{}>", inner.ts());
        }
        Member::Null => text.push_str("null"),
        Member::Undefined => text.push_str("undefined"),
    }
}

fn member_predicate(member: &Member, value: &str) -> String {
    match member {
        Member::StrLit(literal) => format!("{value} === \"{literal}\""),
        Member::NumLit(Num(number)) => format!("{value} === {}", *number as i64),
        Member::True => format!("{value} === true"),
        Member::False => format!("{value} === false"),
        Member::Number => format!("typeof {value} === \"number\""),
        Member::String => format!("typeof {value} === \"string\""),
        Member::Boolean => format!("typeof {value} === \"boolean\""),
        Member::BigInt => format!("typeof {value} === \"bigint\""),
        Member::Promise(_) => format!("__ut.isPromise({value})"),
        Member::Null => format!("{value} === null"),
        Member::Undefined => format!("{value} === void 0"),
    }
}

fn normalized(members: Vec<Member>) -> Ty {
    let mut promise: Option<Ty> = None;
    let mut rest: Vec<Member> = Vec::with_capacity(members.len());
    for member in members {
        match member {
            Member::Promise(inner) => {
                promise = Some(match promise {
                    Some(held) => held.join(&inner),
                    None => *inner,
                });
            }
            other => rest.push(other),
        }
    }
    let literals = rest.iter().filter(|member| member.is_literal()).count();
    if literals > MAX_LITERALS {
        rest = rest.iter().map(Member::widened).collect();
    }
    if rest.contains(&Member::True) && rest.contains(&Member::False) {
        rest.push(Member::Boolean);
    }
    let bases: Vec<Member> = rest
        .iter()
        .filter(|member| !member.is_literal())
        .cloned()
        .collect();
    rest.retain(|member| !member.is_literal() || !bases.iter().any(|base| base.covers(member)));
    if let Some(inner) = promise {
        rest.push(Member::Promise(Box::new(inner)));
    }
    rest.sort();
    rest.dedup();
    Ty { members: rest }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    Bottom,
    Known(Ty),
    Top,
}

impl Val {
    pub fn join(&self, other: &Val) -> Val {
        match (self, other) {
            (Val::Top, _) | (_, Val::Top) => Val::Top,
            (Val::Bottom, value) | (value, Val::Bottom) => value.clone(),
            (Val::Known(a), Val::Known(b)) => Val::Known(a.join(b)),
        }
    }

    pub fn ty(&self) -> Option<&Ty> {
        match self {
            Val::Known(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn map(&self, f: impl FnOnce(&Ty) -> Val) -> Val {
        match self {
            Val::Known(ty) => f(ty),
            other => other.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_fold_into_their_base() {
        let ty = Ty::string_literal("a").join(&Ty::string());
        assert_eq!(ty.ts(), "string");
    }

    #[test]
    fn more_than_six_literals_widen() {
        let ty = (0..7).fold(Ty::number_literal(100.0), |held, n| {
            held.join(&Ty::number_literal(f64::from(n)))
        });
        assert_eq!(ty.ts(), "number");
    }

    #[test]
    fn negative_zero_and_fractions_are_never_literals() {
        assert_eq!(Ty::number_literal(-0.0).ts(), "number");
        assert_eq!(Ty::number_literal(0.5).ts(), "number");
        assert_eq!(Ty::number_literal(f64::NAN).ts(), "number");
    }

    #[test]
    fn order_is_stable_with_null_and_undefined_last() {
        let ty = Ty::undefined()
            .join(&Ty::string_literal("b"))
            .join(&Ty::null())
            .join(&Ty::string_literal("a"))
            .join(&Ty::number());
        assert_eq!(ty.ts(), "\"a\" | \"b\" | number | null | undefined");
    }

    #[test]
    fn true_and_false_make_boolean() {
        let ty = Ty::boolean_literal(true).join(&Ty::boolean_literal(false));
        assert_eq!(ty.ts(), "boolean");
    }

    #[test]
    fn unsafe_string_literals_widen() {
        assert_eq!(Ty::string_literal("a\"b").ts(), "string");
        assert_eq!(Ty::string_literal("é").ts(), "string");
    }

    #[test]
    fn containment_respects_literals() {
        let wide = Ty::string();
        let narrow = Ty::string_literal("x");
        assert!(wide.contains(&narrow));
        assert!(!narrow.contains(&wide));
    }
}
