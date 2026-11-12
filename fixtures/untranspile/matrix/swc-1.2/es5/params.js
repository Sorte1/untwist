"use strict";
function greet() {
    var name = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : "anon", greeting = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : "hi " + name;
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
function counted(a) {
    var b = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 2, c = arguments.length > 2 ? arguments[2] : void 0;
    return arguments.length + ":" + a + b + c;
}
console.log(counted(1), counted(1, undefined, 3), counted.length);
var arrow = function() {
    var x = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1;
    for(var _len = arguments.length, ys = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
        ys[_key - 1] = arguments[_key];
    }
    return x + ys.length;
};
console.log(arrow(), arrow(5, 6, 7), arrow.length);
var evals = 0;
function lazy() {
    var v = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : ++evals;
    return v;
}
lazy(1);
lazy();
lazy();
console.log(evals);
function scoped(a) {
    var f = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : function() {
        return a;
    };
    var a = 2;
    return f();
}
console.log(scoped(1));
function withTemps(opts) {
    var size = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 10, label = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : null;
    var ref;
    var width = (ref = opts === null || opts === void 0 ? void 0 : opts.width) !== null && ref !== void 0 ? ref : size;
    return width + ":" + label;
}
console.log(withTemps({
    width: 3
}), withTemps(null, 4, "x"), withTemps.length);
function inlined(a) {
    var flag = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : false, mode = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : "m";
    return [
        a,
        flag,
        mode
    ].join("/");
}
console.log(inlined(1), inlined(1, true), inlined(1, undefined, "z"));
