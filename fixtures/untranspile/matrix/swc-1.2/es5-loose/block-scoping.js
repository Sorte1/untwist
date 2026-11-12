"use strict";
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
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
    var _loop = function(_i) {
        var item = list[_i];
        var handler = function() {
            return item.id;
        };
        if (item.ok) return {
            v: handler()
        };
    };
    for(var _i = 0; _i < list.length; _i++){
        var _ret = _loop(_i);
        if (_typeof(_ret) === "object") return _ret.v;
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
        {
            var s = "case";
            console.log(s);
        }
}
