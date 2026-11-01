"use strict";
var fns = [];
var _loop_1 = function (i) {
    fns.push(function () { return i; });
};
for (var i = 0; i < 3; i++) {
    _loop_1(i);
}
console.log(fns.map(function (f) { return f(); }).join(","));
var fns2 = [];
var _loop_2 = function (k) {
    fns2.push(function () { return k; });
};
for (var k in { a: 1, b: 2 }) {
    _loop_2(k);
}
console.log(fns2.map(function (f) { return f(); }).join(","));
{
    var shadow = "inner";
    console.log(shadow);
}
var obj = {
    name: "o",
    arrow: function () {
        var _this = this;
        return [1].map(function () { return _this.name; });
    },
    plain: function () {
        return [1].map(function () {
            return typeof this;
        });
    },
};
console.log(obj.arrow()[0], obj.plain()[0]);
function argsArrow() {
    var a = function () { return arguments.length; };
    return a(1, 2, 3);
}
console.log(argsArrow(7));
var tpl = function (x) { return "v=".concat(x, " ").concat(typeof x); };
console.log(tpl(1), tpl.name, tpl.length);
var def = function (a, b) {
    if (b === void 0) { b = a + 1; }
    var c = [];
    for (var _i = 2; _i < arguments.length; _i++) {
        c[_i - 2] = arguments[_i];
    }
    return [a, b, c.length];
};
console.log(def(1).join(","), def.length);
try {
    tdz();
    var later_1 = 1;
    function tdz() {
        return later_1;
    }
}
catch (e) {
    console.log(e.constructor.name);
}
var hoisted = typeof laterFn;
function laterFn() { }
console.log(hoisted);
console.log(typeof (function () { }).prototype, typeof function () { }.prototype);
var closures = [];
var j = 0;
var _loop_3 = function () {
    var captured = j;
    closures.push(function () { return captured; });
    j++;
};
while (j < 2) {
    _loop_3();
}
console.log(closures.map(function (f) { return f(); }).join(","));
var short = { a: a, b: function () { return "b"; } };
function a() {
    return "a";
}
console.log(short.a(), short.b(), typeof short.b.prototype);
