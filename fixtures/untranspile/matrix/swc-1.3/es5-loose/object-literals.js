"use strict";
function _define_enumerable_properties(obj, descs) {
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
        _get = function get(target, property, receiver) {
            var base = _super_prop_base(target, property);
            if (!base) return;
            var desc = Object.getOwnPropertyDescriptor(base, property);
            if (desc.get) {
                return desc.get.call(receiver || target);
            }
            return desc.value;
        };
    }
    return _get(target, property, receiver || target);
}
function _get_prototype_of(o) {
    _get_prototype_of = Object.setPrototypeOf ? Object.getPrototypeOf : function getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _get_prototype_of(o);
}
function _super_prop_base(object, property) {
    while(!Object.prototype.hasOwnProperty.call(object, property)){
        object = _get_prototype_of(object);
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
}, _define_enumerable_properties(_obj1, _mutatorMap), _obj1);
console.log(o.x, o.dyn1, o.method(), o.g, o.tdyn, o.inherited, Object.keys(o).join(","));
var sup = {
    hello: function hello() {
        return "base";
    }
};
var sub = _obj = {
    __proto__: sup,
    hello: function hello() {
        return _get(_get_prototype_of(_obj), "hello", this).call(this) + "+sub";
    }
};
console.log(sub.hello());
