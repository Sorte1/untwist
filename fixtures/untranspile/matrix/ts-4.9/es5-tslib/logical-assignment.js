(function () {
    'use strict';

    var _a, _b, _c, _d, _e;
    var _f, _g, _h, _j, _k, _l;
    var o = { a: 0, b: 1, c: null, d: "x" };
    o.a || (o.a = 10);
    o.b && (o.b = 20);
    (_a = o.c) !== null && _a !== void 0 ? _a : (o.c = 30);
    (_b = o.d) !== null && _b !== void 0 ? _b : (o.d = "never");
    console.log(JSON.stringify(o));
    var x = null;
    x !== null && x !== void 0 ? x : (x = 5);
    var y = 0;
    y || (y = 6);
    var z = 1;
    z && (z = 7);
    console.log(x, y, z);
    var hits = 0;
    var target = {
        get k() {
            hits++;
            return 1;
        },
        set k(v) {
            hits += 100;
        },
    };
    target.k || (target.k = 2);
    target.k && (target.k = 3);
    console.log(hits);
    var arr = [null, 0];
    var i = 0;
    (_c = arr[_f = i++]) !== null && _c !== void 0 ? _c : (arr[_f] = "filled");
    console.log(arr.join(","), i);
    var deep = { l: { s: null, t: 0 }, m: { u: 1 } };
    var keyCalls = 0;
    var key = function () {
        keyCalls++;
        return "u";
    };
    ((_d = (_g = deep.l).s) !== null && _d !== void 0 ? _d : (_g.s = [])).push("pushed");
    (_h = deep.l).t || (_h.t = "t");
    (_j = deep.m)[_k = key()] && (_j[_k] = "u2");
    var got = ((_e = (_l = deep.l).n) !== null && _e !== void 0 ? _e : (_l.n = "made"));
    console.log(JSON.stringify(deep), keyCalls, got);

})();
