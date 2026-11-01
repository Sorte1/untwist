mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::decoded;
use refused_rows::check;

const A_GENERATOR_FUNCTION_IS_TAGGED_GENERATOR_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (Object.prototype.toString.call(function*(){})); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_generator_function_is_tagged_generator_function() {
    check(&decoded(A_GENERATOR_FUNCTION_IS_TAGGED_GENERATOR_FUNCTION, "s0|[object GeneratorFunction]\ns1|[object GeneratorFunction]\ns2|[object GeneratorFunction]\n"));
}

const AN_ASYNC_FUNCTION_IS_TAGGED_ASYNC_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (Object.prototype.toString.call(async function(){})); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn an_async_function_is_tagged_async_function() {
    check(&decoded(
        AN_ASYNC_FUNCTION_IS_TAGGED_ASYNC_FUNCTION,
        "s0|[object AsyncFunction]\ns1|[object AsyncFunction]\ns2|[object AsyncFunction]\n",
    ));
}

const AN_ASYNC_ARROW_IS_TAGGED_ASYNC_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (Object.prototype.toString.call(async () => 1)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn an_async_arrow_is_tagged_async_function() {
    check(&decoded(
        AN_ASYNC_ARROW_IS_TAGGED_ASYNC_FUNCTION,
        "s0|[object AsyncFunction]\ns1|[object AsyncFunction]\ns2|[object AsyncFunction]\n",
    ));
}

const AN_ASYNC_GENERATOR_IS_TAGGED_ASYNC_GENERATOR_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return [Object.prototype.toString.call(async function*(){}), Object.prototype.toString.call(function(){}), Object.prototype.toString.call(() => 1)].join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn an_async_generator_is_tagged_async_generator_function() {
    check(&decoded(AN_ASYNC_GENERATOR_IS_TAGGED_ASYNC_GENERATOR_FUNCTION, "s0|[object AsyncGeneratorFunction],[object Function],[object Function]\ns1|[object AsyncGeneratorFunction],[object Function],[object Function]\ns2|[object AsyncGeneratorFunction],[object Function],[object Function]\n"));
}
