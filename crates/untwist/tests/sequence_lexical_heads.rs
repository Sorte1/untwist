mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::agrees;
use refused_rows::check;

const KEEPS_SEQUENCE_FOR_OF_HEAD_TDZ: &str = r#"var x = 'outer';
try { for (let x of (x, [1])) {} console.log('no'); } catch (e) { console.log('tdz'); }"#;

#[test]
fn keeps_sequence_for_of_head_tdz() {
    check(&agrees(KEEPS_SEQUENCE_FOR_OF_HEAD_TDZ, "tdz\n"));
}

const KEEPS_SEQUENCE_FOR_LET_INIT_TDZ: &str = r#"var i = 'o';
try { for (let i = (i, 0); i < 1; i++) {} console.log('no'); } catch (e) { console.log('tdz'); }"#;

#[test]
fn keeps_sequence_for_let_init_tdz() {
    check(&agrees(KEEPS_SEQUENCE_FOR_LET_INIT_TDZ, "tdz\n"));
}

const KEEPS_SEQUENCE_FOR_CONST_OF_HEAD_TDZ: &str = r#"var x = 1;
try { for (const x of (x, [1])) {} console.log('no'); } catch (e) { console.log('tdz'); }"#;

#[test]
fn keeps_sequence_for_const_of_head_tdz() {
    check(&agrees(KEEPS_SEQUENCE_FOR_CONST_OF_HEAD_TDZ, "tdz\n"));
}

const KEEPS_SEQUENCE_FOR_OF_HEAD_MEMBER_READ_TDZ: &str = r#"var o = {k: [1]};
try { for (let o of (o.k, [2])) {} console.log('no'); } catch (e) { console.log('tdz'); }"#;

#[test]
fn keeps_sequence_for_of_head_member_read_tdz() {
    check(&agrees(KEEPS_SEQUENCE_FOR_OF_HEAD_MEMBER_READ_TDZ, "tdz\n"));
}

const KEEPS_SEQUENCE_FOR_IN_HEAD_CALL_TDZ: &str = r#"function id(v) { return v; }
var y = 1;
try { for (let y in (id(y), {a: 1})) {} console.log('no'); } catch (e) { console.log('tdz'); }"#;

#[test]
fn keeps_sequence_for_in_head_call_tdz() {
    check(&agrees(KEEPS_SEQUENCE_FOR_IN_HEAD_CALL_TDZ, "tdz\n"));
}

const KEEPS_SEQUENCE_FOR_LET_INIT_ASSIGN_SHADOW: &str = r#"var k = 5, seen;
try { for (let k = (seen = typeof k, 0); k < 1; k++) {} console.log(seen); } catch (e) { console.log('tdz'); }"#;

#[test]
fn keeps_sequence_for_let_init_assign_shadow() {
    check(&agrees(KEEPS_SEQUENCE_FOR_LET_INIT_ASSIGN_SHADOW, "tdz\n"));
}
