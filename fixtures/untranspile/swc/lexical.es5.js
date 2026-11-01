function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var _loop = function(i) {
    fns.push(function() {
        return i;
    });
}, _loop1 = function(k) {
    fns2.push(function() {
        return k;
    });
}, _loop2 = function() {
    var captured = j;
    closures.push(function() {
        return captured;
    });
    j++;
};
var fns = [];
for(var i = 0; i < 3; i++)_loop(i);
console.log(fns.map(function(f) {
    return f();
}).join(","));
var fns2 = [];
for(var k in {
    a: 1,
    b: 2
})_loop1(k);
console.log(fns2.map(function(f) {
    return f();
}).join(","));
{
    var shadow = "inner";
    console.log(shadow);
}var obj = {
    name: "o",
    arrow: function arrow() {
        var _this = this;
        return [
            1
        ].map(function() {
            return _this.name;
        });
    },
    plain: function plain() {
        return [
            1
        ].map(function() {
            return _type_of(this);
        });
    }
};
console.log(obj.arrow()[0], obj.plain()[0]);
function argsArrow() {
    var _arguments = arguments;
    var _$a = function a() {
        return _arguments.length;
    };
    return _$a(1, 2, 3);
}
console.log(argsArrow(7));
var tpl = function tpl(x) {
    return "v=".concat(x, " ").concat(typeof x === "undefined" ? "undefined" : _type_of(x));
};
console.log(tpl(1), tpl.name, tpl.length);
var def = function def(a) {
    var b = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : a + 1;
    for(var _len = arguments.length, c = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++){
        c[_key - 2] = arguments[_key];
    }
    return [
        a,
        b,
        c.length
    ];
};
console.log(def(1).join(","), def.length);
try {
    var tdz = function tdz() {
        return later;
    };
    tdz();
    var later = 1;
} catch (e) {
    console.log(e.constructor.name);
}
var hoisted = typeof laterFn === "undefined" ? "undefined" : _type_of(laterFn);
function laterFn() {}
console.log(hoisted);
console.log(_type_of((function() {}).prototype), _type_of((function() {}).prototype));
var closures = [];
var j = 0;
while(j < 2)_loop2();
console.log(closures.map(function(f) {
    return f();
}).join(","));
var short = {
    a: a,
    b: function b() {
        return "b";
    }
};
function a() {
    return "a";
}
console.log(short.a(), short.b(), _type_of(short.b.prototype));
