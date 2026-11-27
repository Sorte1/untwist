function __p_A_flat_peek(__p_O_flat_object,[]){return Object.keys(__p_O_flat_object).join(",")}
var total=1;
function peek(...__p_a_args){var __p_O_flat_object={get"s1"(){return total}};return __p_A_flat_peek(__p_O_flat_object,__p_a_args)}
console.log(peek());
