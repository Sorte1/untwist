"use strict";
function _defineProperty(obj, key, value) {
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
function _objectSpread(target) {
    for(var i = 1; i < arguments.length; i++){
        var source = arguments[i] != null ? arguments[i] : {};
        var ownKeys = Object.keys(source);
        if (typeof Object.getOwnPropertySymbols === 'function') {
            ownKeys = ownKeys.concat(Object.getOwnPropertySymbols(source).filter(function(sym) {
                return Object.getOwnPropertyDescriptor(source, sym).enumerable;
            }));
        }
        ownKeys.forEach(function(key) {
            _defineProperty(target, key, source[key]);
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
function _objectSpreadProps(target, source) {
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
function _objectWithoutProperties(source, excluded) {
    if (source == null) return {};
    var target = _objectWithoutPropertiesLoose(source, excluded);
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
function _objectWithoutPropertiesLoose(source, excluded) {
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
function _toPrimitive(input, hint) {
    if (_typeof(input) !== "object" || input === null) return input;
    var prim = input[Symbol.toPrimitive];
    if (prim !== undefined) {
        var res = prim.call(input, hint || "default");
        if (_typeof(res) !== "object") return res;
        throw new TypeError("@@toPrimitive must return a primitive value.");
    }
    return (hint === "string" ? String : Number)(input);
}
function _toPropertyKey(arg) {
    var key = _toPrimitive(arg, "string");
    return _typeof(key) === "symbol" ? key : String(key);
}
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
const base = {
    a: 1,
    b: 2
};
const extra = {
    c: 3
};
const merged = _objectSpreadProps(_objectSpread({}, base, extra), {
    d: 4
});
const overridden = _objectSpreadProps(_objectSpread({}, base), {
    a: 9
});
console.log(JSON.stringify(merged), JSON.stringify(overridden));
const { a  } = merged, others = _objectWithoutProperties(merged, [
    "a"
]);
console.log(a, JSON.stringify(others));
function without(_param) {
    var { secret  } = _param, pub = _objectWithoutProperties(_param, [
        "secret"
    ]);
    return pub;
}
console.log(JSON.stringify(without({
    secret: 1,
    open: 2
})));
const withGetter = _objectSpread({}, {
    get g () {
        return "got";
    }
});
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
const sym = Symbol("s");
const symObj = _objectSpread({}, {
    [sym]: 1,
    plain: 2
});
console.log(symObj[sym], symObj.plain);
const { [sym]: picked  } = symObj, restSym = _objectWithoutProperties(symObj, [
    sym
].map(_toPropertyKey));
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_objectSpread({}, null, undefined, "hi", [
    5
])));
let order = [];
const tracked = {
    get x () {
        order.push("x");
        return 1;
    },
    get y () {
        order.push("y");
        return 2;
    }
};
const copy = _objectSpread({}, tracked);
console.log(order.join(","), copy.x + copy.y);
