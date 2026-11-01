function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_with_holes(arr) {
    if (Array.isArray(arr)) return arr;
}
function _iterable_to_array_limit(arr, i) {
    var _i = arr == null ? null : typeof Symbol !== "undefined" && arr[Symbol.iterator] || arr["@@iterator"];
    if (_i == null) return;
    var _arr = [];
    var _n = true;
    var _d = false;
    var _s, _e;
    try {
        for(_i = _i.call(arr); !(_n = (_s = _i.next()).done); _n = true){
            _arr.push(_s.value);
            if (i && _arr.length === i) break;
        }
    } catch (err) {
        _d = true;
        _e = err;
    } finally{
        try {
            if (!_n && _i["return"] != null) _i["return"]();
        } finally{
            if (_d) throw _e;
        }
    }
    return _arr;
}
function _non_iterable_rest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _sliced_to_array(arr, i) {
    return _array_with_holes(arr) || _iterable_to_array_limit(arr, i) || _unsupported_iterable_to_array(arr, i) || _non_iterable_rest();
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
var _ref = [
    1,
    2,
    undefined,
    4,
    5
], p = _ref[0], tmp = _ref[2], q = tmp === void 0 ? 7 : tmp, r = _ref.slice(3);
console.log(p, q, r.join(","));
var _ref1 = _sliced_to_array(new Set([
    "s1",
    "s2",
    "s3"
]), 2), m = _ref1[0], n = _ref1[1];
console.log(m, n);
var _ref2 = {
    c: undefined,
    k1: "kv"
}, tmp1 = _ref2.a, _ref3 = tmp1 === void 0 ? {
    b: "dflt"
} : tmp1, deep = _ref3.b, _ref_c = _ref2.c, c = _ref_c === void 0 ? 3 : _ref_c, computed = _ref2["k" + 1];
console.log(deep, c, computed);
function f() {
    var _ref = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : {}, x = _ref.x, _ref_y = _ref.y, y = _ref_y === void 0 ? 2 : _ref_y, _ref1 = _sliced_to_array(arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : [
        9
    ], 1), z = _ref1[0];
    return [
        x,
        y,
        z
    ].join("/");
}
console.log(f(), f({
    x: 1
}, [
    5
]), f.length);
var u, v;
var ref, ref1;
ref1 = _sliced_to_array((ref = [
    1,
    2
], v = ref[0], u = ref[1], ref), 2), u = ref1[0], v = ref1[1], ref1;
console.log(u, v);
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    for(var _iterator = Object.entries({
        e1: 1,
        e2: 2
    })[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
        var _step_value = _sliced_to_array(_step.value, 2), key = _step_value[0], value = _step_value[1];
        console.log(key, value);
    }
} catch (err) {
    _didIteratorError = true;
    _iteratorError = err;
} finally{
    try {
        if (!_iteratorNormalCompletion && _iterator.return != null) {
            _iterator.return();
        }
    } finally{
        if (_didIteratorError) {
            throw _iteratorError;
        }
    }
}
var order = [];
var src = {
    get first () {
        order.push("first");
        return 1;
    },
    get second () {
        order.push("second");
        return 2;
    }
};
var second = src.second, first = src.first;
console.log(order.join(","), first, second);
try {
    var _ref4 = _sliced_to_array(null, 1), t = _ref4[0];
    console.log(t);
} catch (e) {
    console.log(e.constructor.name);
}
try {
    var t2 = undefined.t2;
    console.log(t2);
} catch (e) {
    console.log(e.constructor.name);
}
var str = "ab";
var _str = _sliced_to_array(str, 2), c1 = _str[0], c2 = _str[1];
console.log(c1, c2);
var swap = {
    l: "L",
    r: "R"
};
var ref2;
ref2 = swap, swap.r = ref2.l, swap.l = ref2.r, ref2;
console.log(swap.l, swap.r);
var length = "four".length;
console.log(length);
var n1 = 1, n2 = 2, _ref5 = {
    o: [
        3
    ]
}, _ref_o = _sliced_to_array(_ref5.o, 1), n3 = _ref_o[0];
console.log(n1 + n2 + n3);
