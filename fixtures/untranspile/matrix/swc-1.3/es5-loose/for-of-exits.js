"use strict";
function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
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
    var _obj;
    return _obj = {}, _obj[Symbol.iterator] = function() {
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
    }, _obj;
}
for(var _iterator = _create_for_of_iterator_helper_loose(counted("all", [
    1,
    2
])), _step; !(_step = _iterator()).done;){
    var v = _step.value;
    log.push("all=" + v);
}
for(var _iterator1 = _create_for_of_iterator_helper_loose(counted("brk", [
    1,
    2,
    3
])), _step1; !(_step1 = _iterator1()).done;){
    var v1 = _step1.value;
    if (v1 === 2) break;
    log.push("brk=" + v1);
}
function early() {
    for(var _iterator = _create_for_of_iterator_helper_loose(counted("ret", [
        5,
        6,
        7
    ])), _step; !(_step = _iterator()).done;){
        var v = _step.value;
        if (v === 6) return v;
    }
    return -1;
}
log.push("early=" + early());
try {
    for(var _iterator2 = _create_for_of_iterator_helper_loose(counted("thr", [
        1,
        2
    ])), _step2; !(_step2 = _iterator2()).done;){
        var v2 = _step2.value;
        if (v2 === 2) throw new Error("boom " + v2);
    }
} catch (err) {
    log.push("caught " + err.message);
}
var skipped = 0;
for(var _iterator3 = _create_for_of_iterator_helper_loose(counted("cont", [
    1,
    2,
    3
])), _step3; !(_step3 = _iterator3()).done;){
    var v3 = _step3.value;
    if (v3 === 2) continue;
    skipped += v3;
}
log.push("cont=" + skipped);
outer: for(var _iterator4 = _create_for_of_iterator_helper_loose(counted("outer", [
    1,
    2
])), _step4; !(_step4 = _iterator4()).done;){
    var a = _step4.value;
    for(var _iterator5 = _create_for_of_iterator_helper_loose(counted("inner", [
        10,
        20
    ])), _step5; !(_step5 = _iterator5()).done;){
        var b = _step5.value;
        if (b === 20) continue outer;
        log.push(a + b);
    }
}
var last;
for(var _iterator6 = _create_for_of_iterator_helper_loose(counted("assign", [
    7,
    8
])), _step6; !(_step6 = _iterator6()).done;){
    last = _step6.value;
    log.push("seen " + last);
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
for(var _iterator7 = _create_for_of_iterator_helper_loose(pairs), _step7; !(_step7 = _iterator7()).done;){
    var _step_value = _step7.value, key = _step_value[0], value = _step_value[1];
    log.push(key + value);
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
for(var _iterator8 = _create_for_of_iterator_helper_loose(gen()), _step8; !(_step8 = _iterator8()).done;){
    var v4 = _step8.value;
    if (v4 === 1) break;
}
try {
    for(var _iterator9 = _create_for_of_iterator_helper_loose(counted("bad", [
        1
    ])), _step9; !(_step9 = _iterator9()).done;){
        var v5 = _step9.value;
        log.push(v5.missing.field);
    }
} catch (err) {
    log.push(err.constructor.name);
}
var spreadSet = [].concat(new Set([
    4,
    4,
    5
]));
var _keys = new Map([
    [
        "k",
        1
    ]
]).keys(), firstKey = _keys[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
