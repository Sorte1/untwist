"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _create_for_of_iterator_helper_loose(o, allowArrayLike) {
    var it = typeof Symbol !== "undefined" && o[Symbol.iterator] || o["@@iterator"];
    if (it) return (it = it.call(o)).next.bind(it);
    if (Array.isArray(o) || (it = _unsupported_iterable_to_array(o)) || allowArrayLike && o && typeof o.length === "number") {
        if (it) o = it;
        var i = 0;
        return function() {
            if (i >= o.length) return {
                done: true
            };
            return {
                done: false,
                value: o[i++]
            };
        };
    }
    throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _ts_generator(thisArg, body) {
    var f, y, t, _ = {
        label: 0,
        sent: function() {
            if (t[0] & 1) throw t[1];
            return t[1];
        },
        trys: [],
        ops: []
    }, g = Object.create((typeof Iterator === "function" ? Iterator : Object).prototype), d = Object.defineProperty;
    return d(g, "next", {
        value: verb(0)
    }), d(g, "throw", {
        value: verb(1)
    }), d(g, "return", {
        value: verb(2)
    }), typeof Symbol === "function" && d(g, Symbol.iterator, {
        value: function() {
            return this;
        }
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
        while(g && (g = 0, op[0] && (_ = 0)), _)try {
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
function _ts_values(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) {
        return m.call(o);
    }
    if (o && typeof o.length === "number") {
        return {
            next: function() {
                if (o && i >= o.length) {
                    o = void 0;
                }
                return {
                    value: o && o[i++],
                    done: !o
                };
            }
        };
    }
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(o);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
function range(start, end, step) {
    var i;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                if (step === void 0) step = 1;
                i = start;
                _state.label = 1;
            case 1:
                if (!(i < end)) return [
                    3,
                    4
                ];
                return [
                    4,
                    i
                ];
            case 2:
                _state.sent();
                _state.label = 3;
            case 3:
                i += step;
                return [
                    3,
                    1
                ];
            case 4:
                return [
                    2
                ];
        }
    });
}
function inner() {
    var got;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                return [
                    4,
                    "first"
                ];
            case 1:
                got = _state.sent();
                return [
                    4,
                    "got " + got
                ];
            case 2:
                _state.sent();
                return [
                    2,
                    "inner-done"
                ];
        }
    });
}
function outer() {
    var r;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                return [
                    5,
                    _ts_values(inner())
                ];
            case 1:
                r = _state.sent();
                return [
                    4,
                    r
                ];
            case 2:
                _state.sent();
                _state.label = 3;
            case 3:
                _state.trys.push([
                    3,
                    ,
                    5,
                    7
                ]);
                return [
                    4,
                    "try"
                ];
            case 4:
                _state.sent();
                return [
                    3,
                    7
                ];
            case 5:
                return [
                    4,
                    "cleanup"
                ];
            case 6:
                _state.sent();
                return [
                    7
                ];
            case 7:
                return [
                    2
                ];
        }
    });
}
console.log([].concat(range(0, 5)).join(","), Array.from(range(1, 10, 3)).join(","));
var it = outer();
console.log(JSON.stringify([
    it.next(),
    it.next("x"),
    it.next(),
    it.next(),
    it.return(7),
    it.next(),
    it.next()
]));
var _obj;
var obj = (_obj = {}, _obj[Symbol.iterator] = function() {
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
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
                    2
                ];
        }
    });
}, _obj);
console.log([].concat(obj).join(","));
function thrower() {
    var e;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                _state.trys.push([
                    0,
                    2,
                    ,
                    4
                ]);
                return [
                    4,
                    1
                ];
            case 1:
                _state.sent();
                return [
                    3,
                    4
                ];
            case 2:
                e = _state.sent();
                return [
                    4,
                    "caught " + e
                ];
            case 3:
                _state.sent();
                return [
                    3,
                    4
                ];
            case 4:
                return [
                    2
                ];
        }
    });
}
var t = thrower();
t.next();
console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
function fib() {
    var a, b, ref;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                a = 0, b = 1;
                _state.label = 1;
            case 1:
                if (!true) return [
                    3,
                    3
                ];
                return [
                    4,
                    a
                ];
            case 2:
                _state.sent();
                ref = [
                    b,
                    a + b
                ], a = ref[0], b = ref[1], ref;
                return [
                    3,
                    1
                ];
            case 3:
                return [
                    2
                ];
        }
    });
}
var f = fib();
var first = [];
for(var _iterator = _create_for_of_iterator_helper_loose(f), _step; !(_step = _iterator()).done;){
    var v = _step.value;
    if (v > 20) break;
    first.push(v);
}
console.log(first.join(","));
