function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_with_holes(arr) {
    if (Array.isArray(arr)) return arr;
}
function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else obj[key] = value;
    return obj;
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
function _object_without_properties(source, excluded) {
    if (source == null) return {};
    var target = {}, sourceKeys, key, i;
    if (typeof Reflect !== "undefined" && Reflect.ownKeys) {
        sourceKeys = Reflect.ownKeys(Object(source));
        for(i = 0; i < sourceKeys.length; i++){
            key = sourceKeys[i];
            if (excluded.indexOf(key) >= 0) continue;
            if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
            target[key] = source[key];
        }
        return target;
    }
    target = _object_without_properties_loose(source, excluded);
    if (Object.getOwnPropertySymbols) {
        sourceKeys = Object.getOwnPropertySymbols(source);
        for(i = 0; i < sourceKeys.length; i++){
            key = sourceKeys[i];
            if (excluded.indexOf(key) >= 0) continue;
            if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
            target[key] = source[key];
        }
    }
    return target;
}
function _object_without_properties_loose(source, excluded) {
    if (source == null) return {};
    var target = {}, sourceKeys = Object.getOwnPropertyNames(source), key, i;
    for(i = 0; i < sourceKeys.length; i++){
        key = sourceKeys[i];
        if (excluded.indexOf(key) >= 0) continue;
        if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
        target[key] = source[key];
    }
    return target;
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
var sym = Symbol.for("s");
var src = _define_property({
    a: 1,
    b: 2,
    c: 3
}, sym, 4);
var a = src.a, others = _object_without_properties(src, [
    "a"
]);
console.log(a, Object.keys(others).join(","), Object.getOwnPropertySymbols(others).length);
function pick(_0) {
    var b = _0.b, rest = _object_without_properties(_0, [
        "b"
    ]);
    return b + ":" + Object.keys(rest).join(",");
}
console.log(pick(src));
var c = src["c"], bee = src.b, remaining = _object_without_properties(src, [
    "c",
    "b"
]);
console.log(c, bee, Object.keys(remaining).join(","));
var head = 1, tail = [
    2,
    3,
    4
];
console.log(head, tail.join(","));
var _keys = _sliced_to_array(new Map([
    [
        "k",
        "v"
    ],
    [
        "k2",
        "v2"
    ]
]).keys(), 2), k1 = _keys[0], k2 = _keys[1];
console.log(k1, k2);
var reads = [];
var watched = {
    get x () {
        reads.push("x");
        return 1;
    },
    get y () {
        reads.push("y");
        return 2;
    },
    z: 3
};
var y = watched.y, fromWatched = _object_without_properties(watched, [
    "y"
]);
console.log(y, Object.keys(fromWatched).join(","), reads.join(","));
var box = {
    p: 1,
    q: 2
};
var p1 = box.p;
var q1 = box.q;
console.log(p1, q1);
try {
    var only = null.only;
    console.log(only);
} catch (e) {
    console.log(e.constructor.name);
}
var _xyz = _sliced_to_array("xyz", 2), first = _xyz[0], second = _xyz[1];
console.log(first, second);
var counted = _define_property({}, Symbol.iterator, function() {
    var n = 0;
    return {
        next: function next() {
            n += 1;
            return {
                value: n,
                done: n > 5
            };
        },
        return: function _return() {
            console.log("closed");
            return {};
        }
    };
});
var _counted = _sliced_to_array(counted, 2), one = _counted[0], two = _counted[1];
console.log(one, two);
