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
    if (Object.getOwnPropertyDescriptors) Object.defineProperties(target, Object.getOwnPropertyDescriptors(source));
    else {
        ownKeys(Object(source)).forEach(function(key) {
            Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key));
        });
    }
    return target;
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
function sum() {
    var total = 0;
    for(var i = 0; i < arguments.length; i++)total += arguments[i];
    return total;
}
const a = [
    1,
    2,
    3
];
const b = [
    ...a,
    4,
    ...[
        5,
        6
    ]
];
console.log(b.join(","), b.length);
console.log(sum(...a, 10, ...[
    20
]));
console.log(Math.max(...new Set([
    3,
    9,
    1
])));
const holes = [
    ,
    1
];
const copy = [
    ...holes
];
console.log(0 in copy, copy.length);
const o1 = {
    x: 1,
    y: 2,
    get z () {
        return 3;
    }
};
const o2 = _object_spread_props(_object_spread({}, o1), {
    y: 5,
    w: 6
});
const zd = Object.getOwnPropertyDescriptor(o2, "z");
console.log(JSON.stringify(o2), Object.keys(o2).join(","), zd.enumerable, typeof zd.get);
const s = Symbol("k");
const o3 = {
    [s]: 1,
    a: 2
};
const o4 = _object_spread({}, o3, null, undefined, "pq");
console.log(o4[s], Object.getOwnPropertySymbols(o4).length, JSON.stringify(o4));
function rest(first, ...others) {
    return first + ":" + others.join("|") + ":" + others.length;
}
console.log(rest(1), rest(1, 2, 3), rest.length);
const { x } = o2, others = _object_without_properties(o2, [
    "x"
]);
console.log(x, JSON.stringify(others));
function callsThis(...args) {
    return this.base + args.length;
}
const holder = {
    base: 100,
    m: callsThis
};
console.log(holder.m(...a), holder["m"](...a, ...a));
try {
    console.log([
        ...undefined
    ]);
} catch (e) {
    console.log(e.constructor.name);
}
console.log(new Date(...[
    2020,
    0,
    2
]).getFullYear());
const order = [];
const tap = (v)=>(order.push(v), [
        v
    ]);
console.log(sum(...tap(1), ...tap(2)), order.join(","));
try {
    console.log([
        ..."xy"
    ].join("-"));
} catch (e) {
    console.log(e.constructor.name);
}
