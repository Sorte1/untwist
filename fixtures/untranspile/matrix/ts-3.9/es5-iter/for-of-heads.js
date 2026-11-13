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
var e_1, _a, e_2, _b, e_3, _c, e_4, _d, e_5, _e, e_6, _f, e_7, _g;
var calls = [];
var handlers = [function () { return calls.push("a"); }, function () { return calls.push("b"); }];
try {
    for (var _h = __values(new Set(handlers)), _j = _h.next(); !_j.done; _j = _h.next()) {
        var handler = _j.value;
        handler();
    }
}
catch (e_1_1) { e_1 = { error: e_1_1 }; }
finally {
    try {
        if (_j && !_j.done && (_a = _h.return)) _a.call(_h);
    }
    finally { if (e_1) throw e_1.error; }
}
var counts = {};
try {
    for (var _k = __values(new Set(["x", "y", "x"])), _l = _k.next(); !_l.done; _l = _k.next()) {
        var key = _l.value;
        counts[key] = (counts[key] || 0) + 1;
    }
}
catch (e_2_1) { e_2 = { error: e_2_1 }; }
finally {
    try {
        if (_l && !_l.done && (_b = _k.return)) _b.call(_k);
    }
    finally { if (e_2) throw e_2.error; }
}
try {
    for (var _m = __values(new Set([{ list: [1, 2] }, { list: [3] }])), _o = _m.next(); !_o.done; _o = _m.next()) {
        var entry = _o.value;
        try {
            for (var _p = (e_4 = void 0, __values(entry.list)), _q = _p.next(); !_q.done; _q = _p.next()) {
                var n = _q.value;
                calls.push(n);
            }
        }
        catch (e_4_1) { e_4 = { error: e_4_1 }; }
        finally {
            try {
                if (_q && !_q.done && (_d = _p.return)) _d.call(_p);
            }
            finally { if (e_4) throw e_4.error; }
        }
    }
}
catch (e_3_1) { e_3 = { error: e_3_1 }; }
finally {
    try {
        if (_o && !_o.done && (_c = _m.return)) _c.call(_m);
    }
    finally { if (e_3) throw e_3.error; }
}
var removed = { a: 1, b: 2, c: 3 };
try {
    for (var _r = __values(new Set(["a", "c"])), _s = _r.next(); !_s.done; _s = _r.next()) {
        var key = _s.value;
        delete removed[key];
    }
}
catch (e_5_1) { e_5 = { error: e_5_1 }; }
finally {
    try {
        if (_s && !_s.done && (_e = _r.return)) _e.call(_r);
    }
    finally { if (e_5) throw e_5.error; }
}
var total = 0;
var weights = { p: 2, q: 5 };
try {
    for (var _t = __values(new Set(["p", "q", "r"])), _u = _t.next(); !_u.done; _u = _t.next()) {
        var name = _u.value;
        total += weights[name] || 0;
    }
}
catch (e_6_1) { e_6 = { error: e_6_1 }; }
finally {
    try {
        if (_u && !_u.done && (_f = _t.return)) _f.call(_t);
    }
    finally { if (e_6) throw e_6.error; }
}
try {
    for (var _v = __values(new Set([{ run: function (k) { return calls.push(k); } }])), _w = _v.next(); !_w.done; _w = _v.next()) {
        var item = _w.value;
        item.run("ran");
    }
}
catch (e_7_1) { e_7 = { error: e_7_1 }; }
finally {
    try {
        if (_w && !_w.done && (_g = _v.return)) _g.call(_v);
    }
    finally { if (e_7) throw e_7.error; }
}
console.log(calls.join(), JSON.stringify(counts), JSON.stringify(removed), total);
