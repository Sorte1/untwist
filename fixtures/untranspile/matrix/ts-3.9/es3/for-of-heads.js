"use strict";
var calls = [];
var handlers = [function () { return calls.push("a"); }, function () { return calls.push("b"); }];
for (var _i = 0, _a = new Set(handlers); _i < _a.length; _i++) {
    var handler = _a[_i];
    handler();
}
var counts = {};
for (var _b = 0, _c = new Set(["x", "y", "x"]); _b < _c.length; _b++) {
    var key = _c[_b];
    counts[key] = (counts[key] || 0) + 1;
}
for (var _d = 0, _e = new Set([{ list: [1, 2] }, { list: [3] }]); _d < _e.length; _d++) {
    var entry = _e[_d];
    for (var _f = 0, _g = entry.list; _f < _g.length; _f++) {
        var n = _g[_f];
        calls.push(n);
    }
}
var removed = { a: 1, b: 2, c: 3 };
for (var _h = 0, _j = new Set(["a", "c"]); _h < _j.length; _h++) {
    var key = _j[_h];
    delete removed[key];
}
var total = 0;
var weights = { p: 2, q: 5 };
for (var _k = 0, _l = new Set(["p", "q", "r"]); _k < _l.length; _k++) {
    var name = _l[_k];
    total += weights[name] || 0;
}
for (var _m = 0, _o = new Set([{ run: function (k) { return calls.push(k); } }]); _m < _o.length; _m++) {
    var item = _o[_m];
    item.run("ran");
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
