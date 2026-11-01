function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
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
    } else obj[key] = value;
    return obj;
}
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) {
        return Array.from(iter);
    }
}
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _to_consumable_array(arr) {
    return _array_without_holes(arr) || _iterable_to_array(arr) || _unsupported_iterable_to_array(arr) || _non_iterable_spread();
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
function counter(n) {
    var i, got;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                _state.trys.push([
                    0,
                    ,
                    5,
                    6
                ]);
                i = 0;
                _state.label = 1;
            case 1:
                if (!(i < n)) return [
                    3,
                    4
                ];
                return [
                    4,
                    i
                ];
            case 2:
                got = _state.sent();
                if (got) console.log("got", got);
                _state.label = 3;
            case 3:
                i++;
                return [
                    3,
                    1
                ];
            case 4:
                return [
                    2,
                    "done"
                ];
            case 5:
                console.log("cleanup");
                return [
                    7
                ];
            case 6:
                return [
                    2
                ];
        }
    });
}
var it = counter(3);
console.log(JSON.stringify(it.next()), JSON.stringify(it.next("a")));
console.log(JSON.stringify(it.return("early")), JSON.stringify(it.next()));
console.log(_to_consumable_array(counter(2)).join(","));
function deleg() {
    var r;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                return [
                    5,
                    _ts_values(counter(1))
                ];
            case 1:
                r = _state.sent();
                return [
                    4,
                    "r=" + r
                ];
            case 2:
                _state.sent();
                return [
                    2
                ];
        }
    });
}
console.log(_to_consumable_array(deleg()).join(","));
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
                    4,
                    3
                ];
            case 5:
                _state.sent();
                return [
                    2
                ];
        }
    });
}
var t = thrower();
t.next();
console.log(t.throw("x").value, t.next().value, t.next().done);
function labelled() {
    var _i, _iter, a, _i1, _iter1, b;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                _i = 0, _iter = [
                    1,
                    2
                ];
                _state.label = 1;
            case 1:
                if (!(_i < _iter.length)) return [
                    3,
                    6
                ];
                a = _iter[_i];
                _i1 = 0, _iter1 = [
                    1,
                    2
                ];
                _state.label = 2;
            case 2:
                if (!(_i1 < _iter1.length)) return [
                    3,
                    5
                ];
                b = _iter1[_i1];
                if (b === 2) return [
                    3,
                    5
                ];
                return [
                    4,
                    a * 10 + b
                ];
            case 3:
                _state.sent();
                _state.label = 4;
            case 4:
                _i1++;
                return [
                    3,
                    2
                ];
            case 5:
                _i++;
                return [
                    3,
                    1
                ];
            case 6:
                return [
                    2
                ];
        }
    });
}
console.log(_to_consumable_array(labelled()).join(","));
var gen = _define_property({}, Symbol.iterator, function() {
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
});
console.log(_to_consumable_array(gen).join(""));
console.log(typeof counter === "undefined" ? "undefined" : _type_of(counter), counter.name);
try {
    var g2 = counter(1);
    g2.next();
    g2.throw(new TypeError("t"));
} catch (e) {
    console.log(e.constructor.name);
}
function fib() {
    var x, y, ref;
    return _ts_generator(this, function(_state) {
        switch(_state.label){
            case 0:
                x = 0, y = 1;
                _state.label = 1;
            case 1:
                if (!true) return [
                    3,
                    3
                ];
                return [
                    4,
                    x
                ];
            case 2:
                _state.sent();
                ref = [
                    y,
                    x + y
                ], x = ref[0], y = ref[1], ref;
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
var firsts = [];
var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
try {
    for(var _iterator = fib()[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
        var v = _step.value;
        if (v > 20) break;
        firsts.push(v);
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
console.log(firsts.join(","));
