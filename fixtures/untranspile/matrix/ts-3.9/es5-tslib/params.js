(function () {
    'use strict';

    function greet(name, greeting) {
        if (name === void 0) { name = "anon"; }
        if (greeting === void 0) { greeting = "hi " + name; }
        return greeting + "!";
    }
    console.log(greet(), greet("bo"), greet(undefined, "yo"), greet(null));
    function collect(first) {
        var rest = [];
        for (var _i = 1; _i < arguments.length; _i++) {
            rest[_i - 1] = arguments[_i];
        }
        return first + ":" + rest.join(",") + ":" + rest.length;
    }
    console.log(collect(1), collect(1, 2, 3));
    function counted(a, b, c) {
        if (b === void 0) { b = 2; }
        return arguments.length + ":" + a + b + c;
    }
    console.log(counted(1), counted(1, undefined, 3), counted.length);
    var arrow = function (x) {
        if (x === void 0) { x = 1; }
        var ys = [];
        for (var _i = 1; _i < arguments.length; _i++) {
            ys[_i - 1] = arguments[_i];
        }
        return x + ys.length;
    };
    console.log(arrow(), arrow(5, 6, 7), arrow.length);
    var evals = 0;
    function lazy(v) {
        if (v === void 0) { v = ++evals; }
        return v;
    }
    lazy(1);
    lazy();
    lazy();
    console.log(evals);
    function scoped(a, f) {
        if (f === void 0) { f = function () { return a; }; }
        var a = 2;
        return f();
    }
    console.log(scoped(1));
    function withTemps(opts, size, label) {
        var _a;
        if (size === void 0) { size = 10; }
        if (label === void 0) { label = null; }
        var width = (_a = opts === null || opts === void 0 ? void 0 : opts.width) !== null && _a !== void 0 ? _a : size;
        return width + ":" + label;
    }
    console.log(withTemps({ width: 3 }), withTemps(null, 4, "x"), withTemps.length);
    function inlined(a, flag, mode) {
        if (flag === void 0) { flag = false; }
        if (mode === void 0) { mode = "m"; }
        return [a, flag, mode].join("/");
    }
    console.log(inlined(1), inlined(1, true), inlined(1, undefined, "z"));

})();
