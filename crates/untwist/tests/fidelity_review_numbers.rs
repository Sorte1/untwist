mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::q_refused;
use refused_rows::check;

const PARSE_INT_IN_RADIX_36_ABOVE_TWO_TO_THE_53_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (String(parseInt('9007199254740992', 36))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn parse_int_in_radix_36_above_two_to_the_53_is_refused() {
    check(&q_refused(
        PARSE_INT_IN_RADIX_36_ABOVE_TWO_TO_THE_53_IS_REFUSED,
        "s0|1.9896986116031807e+24\ns1|1.9896986116031807e+24\ns2|1.9896986116031807e+24\n",
    ));
}
