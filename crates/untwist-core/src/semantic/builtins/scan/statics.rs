use oxc_ast::ast::*;

use super::reach::{member_of, Key};
use super::Scan;

const REGEXP: &str = "RegExp";

const LEGACY_STATICS: [&str; 21] = [
    "$1",
    "$2",
    "$3",
    "$4",
    "$5",
    "$6",
    "$7",
    "$8",
    "$9",
    "$_",
    "$&",
    "$+",
    "$`",
    "$'",
    "input",
    "lastMatch",
    "lastParen",
    "leftContext",
    "rightContext",
    "multiline",
    "$*",
];

impl Scan<'_> {
    pub(super) fn judge_statics(&mut self, object: &Expression<'_>, key: Key<'_>) {
        if self.statics
            && !self.writes_only
            && self.may_name_a_static(key)
            && self.may_be_regexp(object)
        {
            self.aliased();
        }
    }

    pub(super) fn judge_member_statics(&mut self, expression: &Expression<'_>) {
        if let Some((object, key)) = member_of(expression) {
            self.judge_statics(object, key);
        }
    }

    fn may_name_a_static(&self, key: Key<'_>) -> bool {
        match self.sharpen(key) {
            Key::Name(name) => LEGACY_STATICS.contains(&name),
            Key::Other => false,
            Key::Unknown(expression) => self
                .known_key(expression)
                .is_none_or(|name| LEGACY_STATICS.contains(&name.as_str())),
        }
    }

    fn may_be_regexp(&self, object: &Expression<'_>) -> bool {
        let object = object.without_parentheses();
        if self.is_global_object(object) || self.is_maybe_global_this(object) {
            return false;
        }
        match object {
            Expression::Identifier(id) => self.global_named(id, &[REGEXP]),
            Expression::ThisExpression(_) => false,
            other => self.is_root(other),
        }
    }
}
