var _o$c, _o$d, _i, _arr$_i, _deep$l, _deep$l$s, _deep$l2, _deep$m, _key, _deep$l3, _deep$l3$n;
const o = {
  a: 0,
  b: 1,
  c: null,
  d: "x"
};
o.a || (o.a = 10);
o.b && (o.b = 20);
(_o$c = o.c) !== null && _o$c !== void 0 ? _o$c : o.c = 30;
(_o$d = o.d) !== null && _o$d !== void 0 ? _o$d : o.d = "never";
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
  get k() {
    hits++;
    return 1;
  },
  set k(v) {
    hits += 100;
  }
};
target.k || (target.k = 2);
target.k && (target.k = 3);
console.log(hits);
const arr = [null, 0];
let i = 0;
(_arr$_i = arr[_i = i++]) !== null && _arr$_i !== void 0 ? _arr$_i : arr[_i] = "filled";
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
const key = () => {
  keyCalls++;
  return "u";
};
((_deep$l$s = (_deep$l = deep.l).s) !== null && _deep$l$s !== void 0 ? _deep$l$s : _deep$l.s = []).push("pushed");
(_deep$l2 = deep.l).t || (_deep$l2.t = "t");
(_deep$m = deep.m)[_key = key()] && (_deep$m[_key] = "u2");
const got = (_deep$l3$n = (_deep$l3 = deep.l).n) !== null && _deep$l3$n !== void 0 ? _deep$l3$n : _deep$l3.n = "made";
console.log(JSON.stringify(deep), keyCalls, got);
