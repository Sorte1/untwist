"use strict";
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var __read = (this && this.__read) || function (o, n) {
    var m = typeof Symbol === "function" && o[Symbol.iterator];
    if (!m) return o;
    var i = m.call(o), r, ar = [], e;
    try {
        while ((n === void 0 || n-- > 0) && !(r = i.next()).done) ar.push(r.value);
    }
    catch (error) { e = { error: error }; }
    finally {
        try {
            if (r && !r.done && (m = i["return"])) m.call(i);
        }
        finally { if (e) throw e.error; }
    }
    return ar;
};
var e_1, _a, e_2, _b, e_3, _c;
var fns = [];
var _loop_1 = function (v) {
    var doubled = v * 2;
    var note = "n" + v;
    fns.push(function () { return doubled + note; });
    note += "!";
};
try {
    for (var _d = __values(new Set([1, 2, 3])), _e = _d.next(); !_e.done; _e = _d.next()) {
        var v = _e.value;
        _loop_1(v);
    }
}
catch (e_1_1) { e_1 = { error: e_1_1 }; }
finally {
    try {
        if (_e && !_e.done && (_a = _d.return)) _a.call(_d);
    }
    finally { if (e_1) throw e_1.error; }
}
console.log(fns.map(function (f) { return f(); }).join(","));
var seen = [];
var _loop_2 = function (key, value) {
    seen.push(function () { return key + value; });
};
try {
    for (var _f = __values(new Map([["a", 1], ["b", 2]])), _g = _f.next(); !_g.done; _g = _f.next()) {
        var _h = __read(_g.value, 2), key = _h[0], value = _h[1];
        _loop_2(key, value);
    }
}
catch (e_2_1) { e_2 = { error: e_2_1 }; }
finally {
    try {
        if (_g && !_g.done && (_b = _f.return)) _b.call(_f);
    }
    finally { if (e_2) throw e_2.error; }
}
console.log(seen.map(function (f) { return f(); }).join(","));
var later = [];
var _loop_3 = function (word) {
    var count = 0;
    later.push(function () { return word + ++count; });
};
try {
    for (var _j = __values("abc"), _k = _j.next(); !_k.done; _k = _j.next()) {
        var word = _k.value;
        _loop_3(word);
    }
}
catch (e_3_1) { e_3 = { error: e_3_1 }; }
finally {
    try {
        if (_k && !_k.done && (_c = _j.return)) _c.call(_j);
    }
    finally { if (e_3) throw e_3.error; }
}
console.log(later.map(function (f) { return f() + f(); }).join(","));
