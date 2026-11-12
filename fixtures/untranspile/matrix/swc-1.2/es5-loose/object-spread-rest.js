"use strict";
function _extends() {
    _extends = Object.assign || function(target) {
        for(var i = 1; i < arguments.length; i++){
            var source = arguments[i];
            for(var key in source){
                if (Object.prototype.hasOwnProperty.call(source, key)) {
                    target[key] = source[key];
                }
            }
        }
        return target;
    };
    return _extends.apply(this, arguments);
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
var base = {
    a: 1,
    b: 2
};
var extra = {
    c: 3
};
var merged = _extends({}, base, extra, {
    d: 4
});
var overridden = _extends({}, base, {
    a: 9
});
console.log(JSON.stringify(merged), JSON.stringify(overridden));
var a = merged.a, others = _objectWithoutPropertiesLoose(merged, [
    "a"
]);
console.log(a, JSON.stringify(others));
function without(_param) {
    var secret = _param.secret, pub = _objectWithoutPropertiesLoose(_param, [
        "secret"
    ]);
    return pub;
}
console.log(JSON.stringify(without({
    secret: 1,
    open: 2
})));
var withGetter = _extends({}, {
    get g () {
        return "got";
    }
});
console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
var sym = Symbol("s");
var _obj;
var symObj = _extends({}, (_obj = {}, _obj[sym] = 1, _obj.plain = 2, _obj));
console.log(symObj[sym], symObj.plain);
var picked = symObj[sym], restSym = _objectWithoutPropertiesLoose(symObj, [
    sym
].map(_toPropertyKey));
console.log(picked, Object.keys(restSym).join(","));
console.log(JSON.stringify(_extends({}, null, undefined, "hi", [
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
var copy = _extends({}, tracked);
console.log(order.join(","), copy.x + copy.y);
