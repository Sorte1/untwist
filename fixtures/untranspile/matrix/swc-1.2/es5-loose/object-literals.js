"use strict";
function _defineEnumerableProperties(obj, descs) {
    for(var key in descs){
        var desc = descs[key];
        desc.configurable = desc.enumerable = true;
        if ("value" in desc) desc.writable = true;
        Object.defineProperty(obj, key, desc);
    }
    if (Object.getOwnPropertySymbols) {
        var objectSymbols = Object.getOwnPropertySymbols(descs);
        for(var i = 0; i < objectSymbols.length; i++){
            var sym = objectSymbols[i];
            var desc = descs[sym];
            desc.configurable = desc.enumerable = true;
            if ("value" in desc) desc.writable = true;
            Object.defineProperty(obj, sym, desc);
        }
    }
    return obj;
}
function _get(target, property, receiver) {
    if (typeof Reflect !== "undefined" && Reflect.get) {
        _get = Reflect.get;
    } else {
        _get = function _get(target, property, receiver) {
            var base = _superPropBase(target, property);
            if (!base) return;
            var desc = Object.getOwnPropertyDescriptor(base, property);
            if (desc.get) {
                return desc.get.call(receiver);
            }
            return desc.value;
        };
    }
    return _get(target, property, receiver || target);
}
function _getPrototypeOf(o) {
    _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _getPrototypeOf(o);
}
function _superPropBase(object, property) {
    while(!Object.prototype.hasOwnProperty.call(object, property)){
        object = _getPrototypeOf(object);
        if (object === null) break;
    }
    return object;
}
var _obj;
var k = "dyn";
var x = 1;
var _obj1, _mutatorMap = {};
var o = (_obj1 = {
    x: x
}, _obj1[k + "1"] = 2, _obj1.method = function method() {
    return "m" + this.x;
}, _mutatorMap["g"] = _mutatorMap["g"] || {}, _mutatorMap["g"].get = function() {
    return "get";
}, _obj1["t" + k] = 3, _obj1.__proto__ = {
    inherited: true
}, _defineEnumerableProperties(_obj1, _mutatorMap), _obj1);
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
    hello: function hello() {
        return "base";
    }
};
var sub = _obj = {
    __proto__: sup,
    hello: function hello() {
        return _get(_getPrototypeOf(_obj), "hello", this).call(this) + "+sub";
    }
};
console.log(sub.hello());
