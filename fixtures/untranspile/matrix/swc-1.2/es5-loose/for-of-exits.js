"use strict";
var __generator = (void 0) && (void 0).__generator || function(thisArg, body) {
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
};
var _loop = function(_i8, _iter7) {
    var v = _iter7[_i8];
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
for(var _i = 0, _iter = counted("all", [
    1,
    2
]); _i < _iter.length; _i++){
    var v = _iter[_i];
    log.push("all=" + v);
}
for(var _i1 = 0, _iter1 = counted("brk", [
    1,
    2,
    3
]); _i1 < _iter1.length; _i1++){
    var v1 = _iter1[_i1];
    if (v1 === 2) break;
    log.push("brk=" + v1);
}
function early() {
    for(var _i = 0, _iter = counted("ret", [
        5,
        6,
        7
    ]); _i < _iter.length; _i++){
        var v = _iter[_i];
        if (v === 6) return v;
    }
    return -1;
}
log.push("early=" + early());
try {
    for(var _i2 = 0, _iter2 = counted("thr", [
        1,
        2
    ]); _i2 < _iter2.length; _i2++){
        var v2 = _iter2[_i2];
        if (v2 === 2) throw new Error("boom " + v2);
    }
} catch (err) {
    log.push("caught " + err.message);
}
var skipped = 0;
for(var _i3 = 0, _iter3 = counted("cont", [
    1,
    2,
    3
]); _i3 < _iter3.length; _i3++){
    var v3 = _iter3[_i3];
    if (v3 === 2) continue;
    skipped += v3;
}
log.push("cont=" + skipped);
outer: for(var _i4 = 0, _iter4 = counted("outer", [
    1,
    2
]); _i4 < _iter4.length; _i4++){
    var a = _iter4[_i4];
    for(var _i5 = 0, _iter5 = counted("inner", [
        10,
        20
    ]); _i5 < _iter5.length; _i5++){
        var b = _iter5[_i5];
        if (b === 20) continue outer;
        log.push(a + b);
    }
}
var last;
for(var _i6 = 0, _iter6 = counted("assign", [
    7,
    8
]); _i6 < _iter6.length; _i6++){
    last = _iter6[_i6];
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
for(var _i7 = 0; _i7 < pairs.length; _i7++){
    var __i = pairs[_i7], key = __i[0], value = __i[1];
    log.push(key + value);
}
var fns = [];
for(var _i8 = 0, _iter7 = [
    1,
    2,
    3
]; _i8 < _iter7.length; _i8++)_loop(_i8, _iter7);
log.push(fns.map(function(f) {
    return f();
}).join(""));
function gen() {
    return __generator(this, function(_state) {
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
for(var _i9 = 0, _iter8 = gen(); _i9 < _iter8.length; _i9++){
    var v4 = _iter8[_i9];
    if (v4 === 1) break;
}
try {
    for(var _i10 = 0, _iter9 = counted("bad", [
        1
    ]); _i10 < _iter9.length; _i10++){
        var v5 = _iter9[_i10];
        log.push(v5.missing.field);
    }
} catch (err1) {
    log.push(err1.constructor.name);
}
var spreadSet = [].concat(new Set([
    4,
    4,
    5
]));
var ref = new Map([
    [
        "k",
        1
    ]
]).keys(), firstKey = ref[0];
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
