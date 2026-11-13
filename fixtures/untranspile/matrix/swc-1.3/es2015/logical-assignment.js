"use strict";
var _o, _o1, _o2, _o3, _target, _target1, _arr, _ref, _deep_l, _deep_l1, _deep_m, _key, _deep_l2;
const o = {
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
let x = null;
x !== null && x !== void 0 ? x : x = 5;
let y = 0;
y || (y = 6);
let z = 1;
z && (z = 7);
console.log(x, y, z);
let hits = 0;
const target = {
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
const arr = [
    null,
    0
];
let i = 0;
var _;
(_ = (_arr = arr)[_ref = i++]) !== null && _ !== void 0 ? _ : _arr[_ref] = "filled";
console.log(arr.join(","), i);
const deep = {
    l: {
        s: null,
        t: 0
    },
    m: {
        u: 1
    }
};
let keyCalls = 0;
const key = ()=>{
    keyCalls++;
    return "u";
};
var _s;
((_s = (_deep_l = deep.l).s) !== null && _s !== void 0 ? _s : _deep_l.s = []).push("pushed");
(_deep_l1 = deep.l).t || (_deep_l1.t = "t");
(_deep_m = deep.m)[_key = key()] && (_deep_m[_key] = "u2");
var _n;
const got = (_n = (_deep_l2 = deep.l).n) !== null && _n !== void 0 ? _n : _deep_l2.n = "made";
console.log(JSON.stringify(deep), keyCalls, got);
