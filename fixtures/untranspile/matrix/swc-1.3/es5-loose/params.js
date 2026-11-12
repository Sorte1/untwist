"use strict";
function greet(name, greeting) {
    if (name === void 0) name = "anon";
    if (greeting === void 0) greeting = "hi " + name;
    return greeting + "!";
}
console.log(greet(), greet("bo"), greet(undefined, "yo"), greet(null));
function collect(first) {
    for(var _len = arguments.length, rest = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
        rest[_key - 1] = arguments[_key];
    }
    return first + ":" + rest.join(",") + ":" + rest.length;
}
console.log(collect(1), collect(1, 2, 3));
function counted(a, b, c) {
    if (b === void 0) b = 2;
    return arguments.length + ":" + a + b + c;
}
console.log(counted(1), counted(1, undefined, 3), counted.length);
var arrow = function(x) {
    if (x === void 0) x = 1;
    for(var _len = arguments.length, ys = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
        ys[_key - 1] = arguments[_key];
    }
    return x + ys.length;
};
console.log(arrow(), arrow(5, 6, 7), arrow.length);
var evals = 0;
function lazy(v) {
    if (v === void 0) v = ++evals;
    return v;
}
lazy(1);
lazy();
lazy();
console.log(evals);
function scoped(a, f) {
    if (f === void 0) f = function() {
        return a;
    };
    var a = 2;
    return f();
}
console.log(scoped(1));
function withTemps(opts, size, label) {
    if (size === void 0) size = 10;
    if (label === void 0) label = null;
    var _opts_width;
    var width = (_opts_width = opts == null ? void 0 : opts.width) != null ? _opts_width : size;
    return width + ":" + label;
}
console.log(withTemps({
    width: 3
}), withTemps(null, 4, "x"), withTemps.length);
function inlined(a, flag, mode) {
    if (flag === void 0) flag = false;
    if (mode === void 0) mode = "m";
    return [
        a,
        flag,
        mode
    ].join("/");
}
console.log(inlined(1), inlined(1, true), inlined(1, undefined, "z"));
