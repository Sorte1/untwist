"use strict";
var _o, _o1, _o2, _o3, _target, _target1, _arr, _ref, _l, _l1, _m, _ref1, _l2;
var o = {
    a: 0,
    b: 1,
    c: null,
    d: "x"
};
(_o = o).a || (_o.a = 10);
(_o1 = o).b && (_o1.b = 20);
var _c;
(_c = (_o2 = o).c) !== null && _c !== void 0 ? _c : _o2.c = 30;
var _d;
(_d = (_o3 = o).d) !== null && _d !== void 0 ? _d : _o3.d = "never";
console.log(JSON.stringify(o));
var x = null;
x !== null && x !== void 0 ? x : x = 5;
var y = 0;
y || (y = 6);
var z = 1;
z && (z = 7);
console.log(x, y, z);
var hits = 0;
var target = {
    get k () {
        hits++;
        return 1;
    },
    set k (v){
        hits += 100;
    }
};
(_target = target).k || (_target.k = 2);
(_target1 = target).k && (_target1.k = 3);
console.log(hits);
var arr = [
    null,
    0
];
var i = 0;
var ref;
(ref = (_arr = arr)[_ref = i++]) !== null && ref !== void 0 ? ref : _arr[_ref] = "filled";
console.log(arr.join(","), i);
var deep = {
    l: {
        s: null,
        t: 0
    },
    m: {
        u: 1
    }
};
var keyCalls = 0;
var key = function() {
    keyCalls++;
    return "u";
};
var _s;
((_s = (_l = deep.l).s) !== null && _s !== void 0 ? _s : _l.s = []).push("pushed");
(_l1 = deep.l).t || (_l1.t = "t");
(_m = deep.m)[_ref1 = key()] && (_m[_ref1] = "u2");
var _n;
var got = (_n = (_l2 = deep.l).n) !== null && _n !== void 0 ? _n : _l2.n = "made";
console.log(JSON.stringify(deep), keyCalls, got);
