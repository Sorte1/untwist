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
var fns = [];
var _loop_1 = function (i) {
    fns.push(function () { return i; });
};
for (var i = 0; i < 3; i++) {
    _loop_1(i);
}
console.log(fns.map(function (f) { return f(); }).join(","));
var later = [];
var _loop_2 = function (i) {
    if (i === 1)
        return "continue";
    if (i === 3)
        return "break";
    later.push(function () {
        return i * 10;
    });
};
for (var i = 0; i < 4; i++) {
    var state_1 = _loop_2(i);
    if (state_1 === "break")
        break;
}
console.log(later.map(function (f) { return f(); }).join(","));
function find(list) {
    var e_1, _a;
    var _loop_5 = function (item) {
        var handler = function () { return item.id; };
        if (item.ok)
            return { value: handler() };
    };
    try {
        for (var list_1 = __values(list), list_1_1 = list_1.next(); !list_1_1.done; list_1_1 = list_1.next()) {
            var item = list_1_1.value;
            var state_2 = _loop_5(item);
            if (typeof state_2 === "object")
                return state_2.value;
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (list_1_1 && !list_1_1.done && (_a = list_1.return)) _a.call(list_1);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return -1;
}
console.log(find([{ id: 1 }, { id: 2, ok: true }]));
var x = "outer";
{
    var x_1 = "inner";
    var y = x_1 + "!";
    console.log(x_1, y);
}
console.log(x);
var ws = [];
var j = 0;
var _loop_3 = function () {
    var k = j;
    ws.push(function () { return k; });
    j++;
};
while (j < 3) {
    _loop_3();
}
console.log(ws.map(function (f) { return f(); }).join(","));
var _loop_4 = function (i, n) {
    setTimeout(function () { return console.log("timer", i); }, 0);
};
for (var i = 0, n = 2; i < n; i++) {
    _loop_4(i, n);
}
switch (1) {
    case 1: {
        var s = "case";
        console.log(s);
    }
}
