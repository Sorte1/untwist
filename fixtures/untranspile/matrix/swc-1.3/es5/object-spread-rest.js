"use strict";
function _define_property(obj, key, value) {
    if (key in obj) {
        Object.defineProperty(obj, key, {
            value: value,
            enumerable: true,
            configurable: true,
            writable: true
        });
    } else {
        obj[key] = value;
    }
    return obj;
}
function _object_spread(target) {
    for(var i = 1; i < arguments.length; i++){
        var source = arguments[i] != null ? arguments[i] : {};
        var ownKeys = Object.keys(source);
        if (typeof Object.getOwnPropertySymbols === "function") {
            ownKeys = ownKeys.concat(Object.getOwnPropertySymbols(source).filter(function(sym) {
                return Object.getOwnPropertyDescriptor(source, sym).enumerable;
            }));
        }
        ownKeys.forEach(function(key) {
            _define_property(target, key, source[key]);
        });
    }
    return target;
}
function ownKeys(object, enumerableOnly) {
    var keys = Object.keys(object);
    if (Object.getOwnPropertySymbols) {
        var symbols = Object.getOwnPropertySymbols(object);
        if (enumerableOnly) {
            symbols = symbols.filter(function(sym) {
                return Object.getOwnPropertyDescriptor(object, sym).enumerable;
            });
        }
        keys.push.apply(keys, symbols);
    }
    return keys;
}
function _object_spread_props(target, source) {
    source = source != null ? source : {};
    if (Object.getOwnPropertyDescriptors) {
        Object.defineProperties(target, Object.getOwnPropertyDescriptors(source));
    } else {
        ownKeys(Object(source)).forEach(function(key) {
            Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key));
        });
    }
    return target;
}
function _object_without_properties(source, excluded) {
    if (source == null) return {};
    var target = _object_without_properties_loose(source, excluded);
    var key, i;
    if (Object.getOwnPropertySymbols) {
        var sourceSymbolKeys = Object.getOwnPropertySymbols(source);
        for(i = 0; i < sourceSymbolKeys.length; i++){
            key = sourceSymbolKeys[i];
            if (excluded.indexOf(key) >= 0) continue;
            if (!Object.prototype.propertyIsEnumerable.call(source, key)) continue;
            target[key] = source[key];
        }
    }
    return target;
}
function _object_without_properties_loose(source, excluded) {
    if (source == null) return {};
    var target = {};
    var sourceKeys = Object.keys(source);
    var key, i;
    for(i = 0; i < sourceKeys.length; i++){
        key = sourceKeys[i];
        if (excluded.indexOf(key) >= 0) continue;
        target[key] = source[key];
    }
    return target;
}
function _to_primitive(input, hint) {
    if (_type_of(input) !== "object" || input === null) return input;
    var prim = input[Symbol.toPrimitive];
    if (prim !== undefined) {
        var res = prim.call(input, hint || "default");
        if (_type_of(res) !== "object") return res;
        throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (hint === "string" ? String : Number)(input);
}
function _to_property_key(arg) {
    var key = _to_primitive(arg, "string");
    return _type_of(key) === "symbol" ? key : String(key);
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var base = {
    a: 1,
    b: 2
};
var extra = {
    c: 3
};
var merged = _object_spread_props(_object_spread({}, base, extra), {
    d: 4
});
var overridden = _object_spread_props(_object_spread({}, base), {
    a: 9
});
console.log(JSON.stringify(merged), JSON.stringify(overridden));
var a = merged.a, others = _object_without_properties(merged, [
    "a"
]);
console.log(a, JSON.stringify(others));
function without(_param) {
    var secret = _param.secret, pub = _object_without_properties(_param, [
        "secret"
    ]);
    return pub;
}
console.log(JSON.stringify(without({
    secret: 1,
    open: 2
})));
var withGetter = _object_spread({}, {
    get g () {
        return "got";
    }
});
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
var sym = Symbol("s");
var _obj;
var symObj = _object_spread({}, (_obj = {}, _define_property(_obj, sym, 1), _define_property(_obj, "plain", 2), _obj));
console.log(symObj[sym], symObj.plain);
var picked = symObj[sym], restSym = _object_without_properties(symObj, [
    sym
].map(_to_property_key));
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_object_spread({}, null, undefined, "hi", [
    5
])));
var order = [];
var tracked = {
    get x () {
        order.push("x");
        return 1;
    },
    get y () {
        order.push("y");
        return 2;
    }
};
var copy = _object_spread({}, tracked);
console.log(order.join(","), copy.x + copy.y);
