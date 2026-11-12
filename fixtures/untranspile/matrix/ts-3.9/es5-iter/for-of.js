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
var e_1, _a, e_2, _b, e_3, _c, e_4, _d, _e, e_5, _f, e_6, _g, e_7, _h;
var out = [];
try {
    for (var _j = __values([1, 2, 3]), _k = _j.next(); !_k.done; _k = _j.next()) {
        var v = _k.value;
        out.push(v * 2);
    }
}
catch (e_1_1) { e_1 = { error: e_1_1 }; }
finally {
    try {
        if (_k && !_k.done && (_a = _j.return)) _a.call(_j);
    }
    finally { if (e_1) throw e_1.error; }
}
try {
    for (var _l = __values("héllo"), _m = _l.next(); !_m.done; _m = _l.next()) {
        var ch = _m.value;
        out.push(ch);
    }
}
catch (e_2_1) { e_2 = { error: e_2_1 }; }
finally {
    try {
        if (_m && !_m.done && (_b = _l.return)) _b.call(_l);
    }
    finally { if (e_2) throw e_2.error; }
}
try {
    for (var _o = __values(new Map([["a", 1], ["b", 2]])), _p = _o.next(); !_p.done; _p = _o.next()) {
        var _q = __read(_p.value, 2), k = _q[0], v = _q[1];
        out.push(k + v);
    }
}
catch (e_3_1) { e_3 = { error: e_3_1 }; }
finally {
    try {
        if (_p && !_p.done && (_c = _o.return)) _c.call(_o);
    }
    finally { if (e_3) throw e_3.error; }
}
try {
    for (var _r = __values(new Set([3, 3, 4])), _s = _r.next(); !_s.done; _s = _r.next()) {
        var v = _s.value;
        out.push(v);
    }
}
catch (e_4_1) { e_4 = { error: e_4_1 }; }
finally {
    try {
        if (_s && !_s.done && (_d = _r.return)) _d.call(_r);
    }
    finally { if (e_4) throw e_4.error; }
}
console.log(out.join(","));
var custom = (_e = {
        from: 1,
        to: 4
    },
    _e[Symbol.iterator] = function () {
        var cur = this.from;
        var last = this.to;
        return {
            next: function () { return (cur <= last ? { value: cur++, done: false } : { value: undefined, done: true }); },
            return: function () {
                out.push("closed");
                return { done: true };
            },
        };
    },
    _e);
var got = [];
try {
    for (var custom_1 = __values(custom), custom_1_1 = custom_1.next(); !custom_1_1.done; custom_1_1 = custom_1.next()) {
        var v = custom_1_1.value;
        if (v === 3)
            break;
        got.push(v);
    }
}
catch (e_5_1) { e_5 = { error: e_5_1 }; }
finally {
    try {
        if (custom_1_1 && !custom_1_1.done && (_f = custom_1.return)) _f.call(custom_1);
    }
    finally { if (e_5) throw e_5.error; }
}
console.log(got.join(","), out[out.length - 1]);
function args() {
    var e_8, _a;
    var xs = [];
    try {
        for (var arguments_1 = __values(arguments), arguments_1_1 = arguments_1.next(); !arguments_1_1.done; arguments_1_1 = arguments_1.next()) {
            var a = arguments_1_1.value;
            xs.push(a);
        }
    }
    catch (e_8_1) { e_8 = { error: e_8_1 }; }
    finally {
        try {
            if (arguments_1_1 && !arguments_1_1.done && (_a = arguments_1.return)) _a.call(arguments_1);
        }
        finally { if (e_8) throw e_8.error; }
    }
    return xs.join("+");
}
console.log(args(1, 2, 3));
var total = 0;
try {
    outer: for (var _t = __values([[1, 2], [3, 4], [5, 6]]), _u = _t.next(); !_u.done; _u = _t.next()) {
        var row = _u.value;
        try {
            for (var row_1 = (e_7 = void 0, __values(row)), row_1_1 = row_1.next(); !row_1_1.done; row_1_1 = row_1.next()) {
                var cell = row_1_1.value;
                if (cell === 4)
                    continue outer;
                if (cell === 6)
                    break outer;
                total += cell;
            }
        }
        catch (e_7_1) { e_7 = { error: e_7_1 }; }
        finally {
            try {
                if (row_1_1 && !row_1_1.done && (_h = row_1.return)) _h.call(row_1);
            }
            finally { if (e_7) throw e_7.error; }
        }
    }
}
catch (e_6_1) { e_6 = { error: e_6_1 }; }
finally {
    try {
        if (_u && !_u.done && (_g = _t.return)) _g.call(_t);
    }
    finally { if (e_6) throw e_6.error; }
}
console.log(total);
