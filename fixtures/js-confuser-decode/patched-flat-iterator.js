var seen=0;var original=Array.prototype[Symbol.iterator];Array.prototype[Symbol.iterator]=function(){seen++;return original.call(this)};
function __p_A_flat_twice(__p_O_flat_object,[x]){return __p_O_flat_object["c1"](x)}
function double(v){return v*2}
function twice(...__p_a_args){var __p_O_flat_object={"c1"(...args){return double(...args)}};return __p_A_flat_twice(__p_O_flat_object,__p_a_args)}
var before=seen;console.log(twice(4),seen-before);
