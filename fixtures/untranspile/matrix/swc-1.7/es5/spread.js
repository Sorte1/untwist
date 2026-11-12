"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_without_holes(arr) {
    if (Array.isArray(arr)) return _array_like_to_array(arr);
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
function _construct(Parent, args, Class) {
    if (_is_native_reflect_construct()) {
        _construct = Reflect.construct;
    } else {
        _construct = function construct(Parent, args, Class) {
            var a = [
                null
            ];
            a.push.apply(a, args);
            var Constructor = Function.bind.apply(Parent, a);
            var instance = new Constructor();
            if (Class) _set_prototype_of(instance, Class.prototype);
            return instance;
        };
    }
    return _construct.apply(null, arguments);
}
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) return Array.from(iter);
}
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
function _to_consumable_array(arr) {
    return _array_without_holes(arr) || _iterable_to_array(arr) || _unsupported_iterable_to_array(arr) || _non_iterable_spread();
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
function _is_native_reflect_construct() {
    try {
        var result = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
    } catch (_) {}
    return (_is_native_reflect_construct = function() {
        return !!result;
    })();
}
function _ts_generator(thisArg, body) {
    var f, y, t, g, _ = {
        label: 0,
        sent: function() {
            if (t[0] & 1) throw t[1];
            return t[1];
        },
        trys: [],
        ops: []
    };
    return g = {
        next: verb(0),
        "throw": verb(1),
        "return": verb(2)
    }, typeof Symbol === "function" && (g[Symbol.iterator] = function() {
        return this;
    }), g;
    function verb(n) {
        return function(v) {
            return step([
                n,
                v
            ]);
        };
    }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while(_)try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [
                op[0] & 2,
                t.value
            ];
            switch(op[0]){
                case 0:
                case 1:
                    t = op;
                    break;
                case 4:
                    _.label++;
                    return {
                        value: op[1],
                        done: false
                    };
                case 5:
                    _.label++;
                    y = op[1];
                    op = [
                        0
                    ];
                    continue;
                case 7:
                    op = _.ops.pop();
                    _.trys.pop();
                    continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) {
                        _ = 0;
                        continue;
                    }
                    if (op[0] === 3 && (!t || op[1] > t[0] && op[1] < t[3])) {
                        _.label = op[1];
                        break;
                    }
                    if (op[0] === 6 && _.label < t[1]) {
                        _.label = t[1];
                        t = op;
                        break;
                    }
                    if (t && _.label < t[2]) {
                        _.label = t[2];
                        _.ops.push(op);
                        break;
                    }
                    if (t[2]) _.ops.pop();
                    _.trys.pop();
                    continue;
            }
            op = body.call(thisArg, _);
        } catch (e) {
            op = [
                6,
                e
            ];
            y = 0;
        } finally{
            f = t = 0;
        }
        if (op[0] & 5) throw op[1];
        return {
            value: op[0] ? op[1] : void 0,
            done: true
        };
    }
}
var _Math, _obj_vals;
function sum() {
    for(var _len = arguments.length, nums = new Array(_len), _key = 0; _key < _len; _key++){
        nums[_key] = arguments[_key];
    }
    return nums.reduce(function(a, b) {
        return a + b;
    }, 0);
}
var a = [
    1,
    2
];
var b = [
    3,
    4
];
console.log(_to_consumable_array(a).concat(_to_consumable_array(b), [
    5
]).join(","), sum.apply(void 0, _to_consumable_array(a).concat(_to_consumable_array(b))), sum.apply(void 0, [
    0
].concat(_to_consumable_array(a))));
function attempt(label, f) {
    try {
        console.log(label, f());
    } catch (e) {
        console.log(label, e.constructor.name);
    }
}
attempt("string", function() {
    return Array.from("abc").join("-");
});
attempt("set", function() {
    return _to_consumable_array(new Set([
        1,
        1,
        2
    ])).join(",");
});
var holes = [
    ,
    1
];
console.log(_to_consumable_array(holes).length, 0 in _to_consumable_array(holes));
console.log((_Math = Math).max.apply(_Math, _to_consumable_array(a).concat(_to_consumable_array(b))));
var Box = function Box() {
    "use strict";
    for(var _len = arguments.length, items = new Array(_len), _key = 0; _key < _len; _key++){
        items[_key] = arguments[_key];
    }
    _class_call_check(this, Box);
    this.items = items;
};
console.log(_construct(Box, _to_consumable_array(a).concat([
    9
])).items.join(","));
var obj = {
    vals: [
        7,
        8
    ],
    total: function total() {
        return sum.apply(void 0, _to_consumable_array(this.vals));
    }
};
console.log(obj.total(), (_obj_vals = obj.vals).concat.apply(_obj_vals, [
    [
        9
    ],
    [
        10
    ]
]).join(","));
function g() {
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                return [
                    4,
                    "x"
                ];
            case 1:
                _state.sent();
                return [
                    4,
                    "y"
                ];
            case 2:
                _state.sent();
                return [
                    2
                ];
        }
    });
}
attempt("generator", function() {
    return _to_consumable_array(g()).concat(_to_consumable_array(g())).join("");
});
