function __p_A_flat_bump(__p_O_flat_object,[step]){__p_O_flat_object["s1"]+=step;return __p_O_flat_object["t1"]+":"+__p_O_flat_object["c1"](__p_O_flat_object["s1"])}
var total=0;function show(v){return "["+v+"]"}
function bump(...__p_a_args){var __p_O_flat_object={get"s1"(){return total},set"s1"(__p_v_value){total=__p_v_value},get"t1"(){return typeof total},"c1"(...args){return show(...args)}};return __p_A_flat_bump(__p_O_flat_object,__p_a_args)}
console.log(bump(2),bump(3),total,bump.length,bump.name);
