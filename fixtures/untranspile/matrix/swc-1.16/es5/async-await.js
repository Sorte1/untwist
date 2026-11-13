"use strict";
function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) {
    try {
        var info = gen[key](arg);
        var value = info.value;
    } catch (error) {
        reject(error);
        return;
    }
    if (info.done) resolve(value);
    else Promise.resolve(value).then(_next, _throw);
}
function _async_to_generator(fn) {
    return function() {
        var self = this, args = arguments;
        return new Promise(function(resolve, reject) {
            var gen = fn.apply(self, args);
            function _next(value) {
                asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value);
            }
            function _throw(err) {
                asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err);
            }
            _next(undefined);
        });
    };
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) throw new TypeError("Cannot call a class as a function");
}
function _defineProperties(target, props) {
    for(var i = 0; i < props.length; i++){
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;
        if ("value" in descriptor) descriptor.writable = true;
        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _create_class(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
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
var log = [];
var later = function later(v) {
    return new Promise(function(resolve) {
        return setTimeout(function() {
            return resolve(v);
        }, 0);
    });
};
function fetchTwice(x) {
    return _async_to_generator(function() {
        var a, b;
        return _ts_generator(this, function(_state) {
            switch(_state.label){
                case 0:
                    log.push("start " + x);
                    return [
                        4,
                        later(x)
                    ];
                case 1:
                    a = _state.sent();
                    return [
                        4,
                        later(a + 1)
                    ];
                case 2:
                    b = _state.sent();
                    return [
                        2,
                        a + b
                    ];
            }
        });
    })();
}
function guarded() {
    return _async_to_generator(function() {
        var e;
        return _ts_generator(this, function(_state) {
            switch(_state.label){
                case 0:
                    _state.trys.push([
                        0,
                        2,
                        3,
                        4
                    ]);
                    return [
                        4,
                        later(0)
                    ];
                case 1:
                    _state.sent();
                    throw new TypeError("t");
                case 2:
                    e = _state.sent();
                    log.push("caught " + e.name);
                    return [
                        2,
                        "r"
                    ];
                case 3:
                    log.push("finally");
                    return [
                        7
                    ];
                case 4:
                    return [
                        2
                    ];
            }
        });
    })();
}
var service = {
    base: 100,
    total: function total(_0) {
        return _async_to_generator(function(n) {
            var _this, _arguments, add, sum, i;
            var _arguments1 = arguments;
            return _ts_generator(this, function(_state) {
                switch(_state.label){
                    case 0:
                        _this = this, _arguments = _arguments1;
                        add = function add(k) {
                            return _async_to_generator(function() {
                                var _arguments = arguments;
                                return _ts_generator(this, function(_state) {
                                    return [
                                        2,
                                        this.base + k + _arguments.length
                                    ];
                                });
                            }).apply(_this, _arguments);
                        };
                        sum = 0;
                        i = 0;
                        _state.label = 1;
                    case 1:
                        if (!(i < n)) return [
                            3,
                            4
                        ];
                        return [
                            4,
                            add(i)
                        ];
                    case 2:
                        sum = sum + _state.sent();
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
                            sum
                        ];
                }
            });
        }).apply(this, arguments);
    }
};
var Loader = /*#__PURE__*/ function() {
    "use strict";
    function Loader() {
        _class_call_check(this, Loader);
        this.items = [];
    }
    _create_class(Loader, [
        {
            key: "load",
            value: function load(v) {
                return _async_to_generator(function() {
                    return _ts_generator(this, function(_state) {
                        switch(_state.label){
                            case 0:
                                return [
                                    4,
                                    null
                                ];
                            case 1:
                                _state.sent();
                                this.items.push(v);
                                return [
                                    2,
                                    this.items.length
                                ];
                        }
                    });
                }).call(this);
            }
        }
    ]);
    return Loader;
}();
function main() {
    return _async_to_generator(function() {
        var p, _, _tmp, l, _1, _tmp1, arrow, _2, _tmp2;
        return _ts_generator(this, function(_state) {
            switch(_state.label){
                case 0:
                    p = fetchTwice(1);
                    log.push("after call");
                    _ = console.log;
                    return [
                        4,
                        p
                    ];
                case 1:
                    _tmp = [
                        _state.sent()
                    ];
                    return [
                        4,
                        guarded()
                    ];
                case 2:
                    _tmp = _tmp.concat([
                        _state.sent()
                    ]);
                    return [
                        4,
                        service.total(3)
                    ];
                case 3:
                    _.apply(console, _tmp.concat([
                        _state.sent()
                    ]));
                    l = new Loader();
                    _1 = console.log;
                    return [
                        4,
                        l.load("a")
                    ];
                case 4:
                    _tmp1 = [
                        _state.sent()
                    ];
                    return [
                        4,
                        l.load("b")
                    ];
                case 5:
                    _1.apply(console, _tmp1.concat([
                        _state.sent()
                    ]));
                    arrow = function arrow(n) {
                        return _async_to_generator(function() {
                            return _ts_generator(this, function(_state) {
                                switch(_state.label){
                                    case 0:
                                        return [
                                            4,
                                            later(n)
                                        ];
                                    case 1:
                                        return [
                                            2,
                                            _state.sent() * 2
                                        ];
                                }
                            });
                        })();
                    };
                    _2 = console.log;
                    return [
                        4,
                        arrow(4)
                    ];
                case 6:
                    _tmp2 = [
                        _state.sent()
                    ];
                    return [
                        4,
                        Promise.all([
                            arrow(1),
                            fetchTwice(2)
                        ])
                    ];
                case 7:
                    _2.apply(console, _tmp2.concat([
                        _state.sent().join(",")
                    ]));
                    console.log(log.join("; "));
                    return [
                        2
                    ];
            }
        });
    })();
}
main().catch(function(e) {
    return console.log("fail", e);
});
