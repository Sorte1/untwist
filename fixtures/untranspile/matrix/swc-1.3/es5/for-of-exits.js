"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_with_holes(arr) {
    if (Array.isArray(arr)) return arr;
}
function _array_without_holes(arr) {
    if (Array.isArray(arr)) return _array_like_to_array(arr);
}
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
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) return Array.from(iter);
}
function _iterable_to_array_limit(arr, i) {
    var _i = arr == null ? null : typeof Symbol !== "undefined" && arr[Symbol.iterator] || arr["@@iterator"];
    if (_i == null) return;
    var _arr = [];
    var _n = true;
    var _d = false;
    var _s, _e;
    try {
        for(_i = _i.call(arr); !(_n = (_s = _i.next()).done); _n = true){
            _arr.push(_s.value);
            if (i && _arr.length === i) break;
        }
    } catch (err) {
        _d = true;
        _e = err;
    } finally{
        try {
            if (!_n && _i["return"] != null) _i["return"]();
        } finally{
            if (_d) throw _e;
        }
    }
    return _arr;
}
function _non_iterable_rest() {
    throw new TypeError("Invalid attempt to destructure non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _sliced_to_array(arr, i) {
    return _array_with_holes(arr) || _iterable_to_array_limit(arr, i) || _unsupported_iterable_to_array(arr, i) || _non_iterable_rest();
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
var _loop = function(_i, _iter) {
    var v = _iter[_i];
    fns.push(function() {
        return v;
    });
};
var log = [];
function counted(name, items) {
    return _define_property({}, Symbol.iterator, function() {
        var at = 0;
        log.push(name + ":open");
        return {
            next: function next() {
                log.push(name + ":next");
                return at < items.length ? {
                    value: items[at++],
                    done: false
                } : {
                    value: undefined,
                    done: true
                };
            },
            return: function _return() {
                log.push(name + ":return");
                return {
                    done: true
                };
            }
        };
    });
}
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    for(var _iterator = counted("all", [
        1,
        2
    ])[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
        var v = _step.value;
        log.push("all=" + v);
    }
} catch (err) {
    _didIteratorError = true;
    _iteratorError = err;
} finally{
    try {
        if (!_iteratorNormalCompletion && _iterator.return != null) {
            _iterator.return();
        }
    } finally{
        if (_didIteratorError) {
            throw _iteratorError;
        }
    }
}
var _iteratorNormalCompletion1 = true, _didIteratorError1 = false, _iteratorError1 = undefined;
try {
    for(var _iterator1 = counted("brk", [
        1,
        2,
        3
    ])[Symbol.iterator](), _step1; !(_iteratorNormalCompletion1 = (_step1 = _iterator1.next()).done); _iteratorNormalCompletion1 = true){
        var v1 = _step1.value;
        if (v1 === 2) break;
        log.push("brk=" + v1);
    }
} catch (err) {
    _didIteratorError1 = true;
    _iteratorError1 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion1 && _iterator1.return != null) {
            _iterator1.return();
        }
    } finally{
        if (_didIteratorError1) {
            throw _iteratorError1;
        }
    }
}
function early() {
    var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
    try {
        for(var _iterator = counted("ret", [
            5,
            6,
            7
        ])[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
            var v = _step.value;
            if (v === 6) return v;
        }
    } catch (err) {
        _didIteratorError = true;
        _iteratorError = err;
    } finally{
        try {
            if (!_iteratorNormalCompletion && _iterator.return != null) {
                _iterator.return();
            }
        } finally{
            if (_didIteratorError) {
                throw _iteratorError;
            }
        }
    }
    return -1;
}
log.push("early=" + early());
try {
    var _iteratorNormalCompletion2 = true, _didIteratorError2 = false, _iteratorError2 = undefined;
    try {
        for(var _iterator2 = counted("thr", [
            1,
            2
        ])[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true){
            var v2 = _step2.value;
            if (v2 === 2) throw new Error("boom " + v2);
        }
    } catch (err) {
        _didIteratorError2 = true;
        _iteratorError2 = err;
    } finally{
        try {
            if (!_iteratorNormalCompletion2 && _iterator2.return != null) {
                _iterator2.return();
            }
        } finally{
            if (_didIteratorError2) {
                throw _iteratorError2;
            }
        }
    }
} catch (err) {
    log.push("caught " + err.message);
}
var skipped = 0;
var _iteratorNormalCompletion3 = true, _didIteratorError3 = false, _iteratorError3 = undefined;
try {
    for(var _iterator3 = counted("cont", [
        1,
        2,
        3
    ])[Symbol.iterator](), _step3; !(_iteratorNormalCompletion3 = (_step3 = _iterator3.next()).done); _iteratorNormalCompletion3 = true){
        var v3 = _step3.value;
        if (v3 === 2) continue;
        skipped += v3;
    }
} catch (err) {
    _didIteratorError3 = true;
    _iteratorError3 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion3 && _iterator3.return != null) {
            _iterator3.return();
        }
    } finally{
        if (_didIteratorError3) {
            throw _iteratorError3;
        }
    }
}
log.push("cont=" + skipped);
var _iteratorNormalCompletion4 = true, _didIteratorError4 = false, _iteratorError4 = undefined;
try {
    outer: for(var _iterator4 = counted("outer", [
        1,
        2
    ])[Symbol.iterator](), _step4; !(_iteratorNormalCompletion4 = (_step4 = _iterator4.next()).done); _iteratorNormalCompletion4 = true){
        var a = _step4.value;
        var _iteratorNormalCompletion5 = true, _didIteratorError5 = false, _iteratorError5 = undefined;
        try {
            for(var _iterator5 = counted("inner", [
                10,
                20
            ])[Symbol.iterator](), _step5; !(_iteratorNormalCompletion5 = (_step5 = _iterator5.next()).done); _iteratorNormalCompletion5 = true){
                var b = _step5.value;
                if (b === 20) continue outer;
                log.push(a + b);
            }
        } catch (err) {
            _didIteratorError5 = true;
            _iteratorError5 = err;
        } finally{
            try {
                if (!_iteratorNormalCompletion5 && _iterator5.return != null) {
                    _iterator5.return();
                }
            } finally{
                if (_didIteratorError5) {
                    throw _iteratorError5;
                }
            }
        }
    }
} catch (err) {
    _didIteratorError4 = true;
    _iteratorError4 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion4 && _iterator4.return != null) {
            _iterator4.return();
        }
    } finally{
        if (_didIteratorError4) {
            throw _iteratorError4;
        }
    }
}
var last;
var _iteratorNormalCompletion6 = true, _didIteratorError6 = false, _iteratorError6 = undefined;
try {
    for(var _iterator6 = counted("assign", [
        7,
        8
    ])[Symbol.iterator](), _step6; !(_iteratorNormalCompletion6 = (_step6 = _iterator6.next()).done); _iteratorNormalCompletion6 = true){
        last = _step6.value;
        log.push("seen " + last);
    }
} catch (err) {
    _didIteratorError6 = true;
    _iteratorError6 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion6 && _iterator6.return != null) {
            _iterator6.return();
        }
    } finally{
        if (_didIteratorError6) {
            throw _iteratorError6;
        }
    }
}
log.push("last=" + last);
var pairs = new Map([
    [
        "x",
        1
    ],
    [
        "y",
        2
    ]
]);
var _iteratorNormalCompletion7 = true, _didIteratorError7 = false, _iteratorError7 = undefined;
try {
    for(var _iterator7 = pairs[Symbol.iterator](), _step7; !(_iteratorNormalCompletion7 = (_step7 = _iterator7.next()).done); _iteratorNormalCompletion7 = true){
        var _step_value = _sliced_to_array(_step7.value, 2), key = _step_value[0], value = _step_value[1];
        log.push(key + value);
    }
} catch (err) {
    _didIteratorError7 = true;
    _iteratorError7 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion7 && _iterator7.return != null) {
            _iterator7.return();
        }
    } finally{
        if (_didIteratorError7) {
            throw _iteratorError7;
        }
    }
}
var fns = [];
for(var _i = 0, _iter = [
    1,
    2,
    3
]; _i < _iter.length; _i++)_loop(_i, _iter);
log.push(fns.map(function(f) {
    return f();
}).join(""));
function gen() {
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                _state.trys.push([
                    0,
                    ,
                    3,
                    4
                ]);
                return [
                    4,
                    1
                ];
            case 1:
                _state.sent();
                return [
                    4,
                    2
                ];
            case 2:
                _state.sent();
                return [
                    3,
                    4
                ];
            case 3:
                log.push("gen:finally");
                return [
                    7
                ];
            case 4:
                return [
                    2
                ];
        }
    });
}
var _iteratorNormalCompletion8 = true, _didIteratorError8 = false, _iteratorError8 = undefined;
try {
    for(var _iterator8 = gen()[Symbol.iterator](), _step8; !(_iteratorNormalCompletion8 = (_step8 = _iterator8.next()).done); _iteratorNormalCompletion8 = true){
        var v4 = _step8.value;
        if (v4 === 1) break;
    }
} catch (err) {
    _didIteratorError8 = true;
    _iteratorError8 = err;
} finally{
    try {
        if (!_iteratorNormalCompletion8 && _iterator8.return != null) {
            _iterator8.return();
        }
    } finally{
        if (_didIteratorError8) {
            throw _iteratorError8;
        }
    }
}
try {
    var _iteratorNormalCompletion9 = true, _didIteratorError9 = false, _iteratorError9 = undefined;
    try {
        for(var _iterator9 = counted("bad", [
            1
        ])[Symbol.iterator](), _step9; !(_iteratorNormalCompletion9 = (_step9 = _iterator9.next()).done); _iteratorNormalCompletion9 = true){
            var v5 = _step9.value;
            log.push(v5.missing.field);
        }
    } catch (err) {
        _didIteratorError9 = true;
        _iteratorError9 = err;
    } finally{
        try {
            if (!_iteratorNormalCompletion9 && _iterator9.return != null) {
                _iterator9.return();
            }
        } finally{
            if (_didIteratorError9) {
                throw _iteratorError9;
            }
        }
    }
} catch (err) {
    log.push(err.constructor.name);
}
var spreadSet = _to_consumable_array(new Set([
    4,
    4,
    5
]));
var _keys = _sliced_to_array(new Map([
    [
        "k",
        1
    ]
]).keys(), 1), firstKey = _keys[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
