function __p_A_flat_name(__p_O_flat_object,[]){return label}
var label="outer";
function outer(){var label="inner";function name(...__p_a_args){var __p_O_flat_object={};return __p_A_flat_name(__p_O_flat_object,__p_a_args)}return name()}
console.log(outer());
