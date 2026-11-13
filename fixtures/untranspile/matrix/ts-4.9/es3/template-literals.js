"use strict";
var __makeTemplateObject = (this && this.__makeTemplateObject) || function (cooked, raw) {
    if (Object.defineProperty) { Object.defineProperty(cooked, "raw", { value: raw }); } else { cooked.raw = raw; }
    return cooked;
};
var name = "world";
var n = 3;
console.log("hello ".concat(name), "".concat(n, " + ").concat(n, " = ").concat(n + n), "nested ".concat("inner ".concat(name)));
console.log("multi\nline", "".concat({ toString: function () { return "obj"; } }), "".concat([1, 2]));
function tag(strings) {
    var values = [];
    for (var _i = 1; _i < arguments.length; _i++) {
        values[_i - 1] = arguments[_i];
    }
    return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag(__makeTemplateObject(["a", "b", "c"], ["a", "b", "c"]), 1, 2), tag(__makeTemplateObject(["\n", "\t"], ["\\n", "\\t"]), "x"));
function same(s) {
    return s;
}
var first = function () { return same(__makeTemplateObject(["x"], ["x"])); };
console.log(first() === first(), Object.isFrozen(first()), String.raw(__makeTemplateObject([void 0, "\n"], ["\\u{41}", "\\n"]), 1));
var sym = { valueOf: function () { return 1; }, toString: function () { return "str"; } };
console.log("".concat(sym), "" + sym);
