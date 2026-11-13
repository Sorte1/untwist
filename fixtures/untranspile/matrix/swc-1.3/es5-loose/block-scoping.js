"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
function _create_for_of_iterator_helper_loose(o, allowArrayLike) {
    var it = typeof Symbol !== "undefined" && o[Symbol.iterator] || o["@@iterator"];
    if (it) return (it = it.call(o)).next.bind(it);
    if (Array.isArray(o) || (it = _unsupported_iterable_to_array(o)) || allowArrayLike && o && typeof o.length === "number") {
        if (it) o = it;
        var i = 0;
        return function() {
            if (i >= o.length) {
                return {
                    done: true
                };
            }
            return {
                done: false,
                value: o[i++]
            };
        };
    }
    throw new TypeError("Invalid attempt to iterate non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
var _loop = function(i) {
    fns.push(function() {
        return i;
    });
}, _loop1 = function(i1) {
    if (i1 === 1) return "continue";
    if (i1 === 3) return "break";
    later.push(function() {
        return i1 * 10;
    });
}, _loop2 = function() {
    var k = j;
    ws.push(function() {
        return k;
    });
    j++;
}, _loop3 = function(i2, n) {
    setTimeout(function() {
        return console.log("timer", i2);
    }, 0);
};
var fns = [];
for(var i = 0; i < 3; i++)_loop(i);
console.log(fns.map(function(f) {
    return f();
}).join(","));
var later = [];
for(var i1 = 0; i1 < 4; i1++){
    var _ret = _loop1(i1);
    if (_ret === "break") break;
}
console.log(later.map(function(f) {
    return f();
}).join(","));
function find(list) {
    var _loop = function() {
        var item = _step.value;
        var handler = function() {
            return item.id;
        };
        if (item.ok) return {
            v: handler()
        };
    };
    for(var _iterator = _create_for_of_iterator_helper_loose(list), _step; !(_step = _iterator()).done;){
        var _ret = _loop();
        if (_type_of(_ret) === "object") return _ret.v;
    }
    return -1;
}
console.log(find([
    {
        id: 1
    },
    {
        id: 2,
        ok: true
    }
]));
var x = "outer";
{
    var x1 = "inner";
    var y = x1 + "!";
    console.log(x1, y);
}console.log(x);
var ws = [];
var j = 0;
while(j < 3)_loop2();
console.log(ws.map(function(f) {
    return f();
}).join(","));
for(var i2 = 0, n = 2; i2 < n; i2++)_loop3(i2, n);
switch(1){
    case 1:
        {
            var s = "case";
            console.log(s);
        }
}
