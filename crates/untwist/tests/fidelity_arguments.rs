mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, q_refused, refused};
use refused_rows::check;

const THE_LEGACY_ARGUMENTS_AND_CALLER_OF_A_FUNCTION_ARE_REFUSE: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + typeof b.arguments + ',' + typeof b.caller; }"##
);

#[test]
fn the_legacy_arguments_and_caller_of_a_function_are_refused() {
    check(&refused(
        THE_LEGACY_ARGUMENTS_AND_CALLER_OF_A_FUNCTION_ARE_REFUSE,
        "s0|object,function\ns1|object,function\ns2|object,function\n",
    ));
}

const CONCAT_DOES_NOT_SPREAD_AN_ARGUMENTS_OBJECT: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + (function(){ return [].concat(arguments).length; })(1, 2); }"##
);

#[test]
fn concat_does_not_spread_an_arguments_object() {
    check(&agrees(
        CONCAT_DOES_NOT_SPREAD_AN_ARGUMENTS_OBJECT,
        "s0|1\ns1|1\ns2|1\n",
    ));
}

const ARRAY_NATIVES_ON_THE_MACHINERY_DECODERS_ARGUMENTS_ARE_REFUSED: &str = qunit!(
    r##"function qb(i, j){ [].reverse.call(arguments); var e=qa(); return e[i] + '|' + j; }"##
);

#[test]
fn array_natives_on_the_machinery_decoders_arguments_are_refused() {
    check(&q_refused(
        ARRAY_NATIVES_ON_THE_MACHINERY_DECODERS_ARGUMENTS_ARE_REFUSED,
        "s0|undefined\ns1|undefined\ns2|undefined\n",
    ));
}

const SPLICE_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(p){ [].splice.call(arguments, 0, 1, 'Z'); return p; })('A')); }"##
);

#[test]
fn splice_on_a_mapped_arguments_object_is_refused() {
    check(&q_refused(
        SPLICE_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED,
        "s0|Z\ns1|Z\ns2|Z\n",
    ));
}

const REVERSE_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(p, q){ [].reverse.call(arguments); return p + q; })('A', 'B')); }"##
);

#[test]
fn reverse_on_a_mapped_arguments_object_is_refused() {
    check(&q_refused(
        REVERSE_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED,
        "s0|BA\ns1|BA\ns2|BA\n",
    ));
}

const SHIFT_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(p, q){ [].shift.call(arguments); return p + q; })('A', 'B')); }"##
);

#[test]
fn shift_on_a_mapped_arguments_object_is_refused() {
    check(&q_refused(
        SHIFT_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED,
        "s0|BB\ns1|BB\ns2|BB\n",
    ));
}

const UNSHIFT_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(p, q){ [].unshift.call(arguments, 'X'); return p + q; })('A', 'B')); }"##
);

#[test]
fn unshift_on_a_mapped_arguments_object_is_refused() {
    check(&q_refused(
        UNSHIFT_ON_A_MAPPED_ARGUMENTS_OBJECT_IS_REFUSED,
        "s0|XA\ns1|XA\ns2|XA\n",
    ));
}

const SPLICE_ON_ARGUMENTS_IN_A_NESTED_HELPER_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ function d(i, k){ [].splice.call(arguments, 1, 1, 'K'); return i + k; } return d('I', 'k'); })()); }"##
);

#[test]
fn splice_on_arguments_in_a_nested_helper_is_refused() {
    check(&q_refused(
        SPLICE_ON_ARGUMENTS_IN_A_NESTED_HELPER_IS_REFUSED,
        "s0|IK\ns1|IK\ns2|IK\n",
    ));
}
