"use strict";
function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) {
    try {
        var info = gen[key](arg);
        var value = info.value;
    } catch (error) {
        reject(error);
        return;
    }
    if (info.done) {
        resolve(value);
    } else {
        Promise.resolve(value).then(_next, _throw);
    }
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
const log = [];
const later = (v)=>new Promise((resolve)=>setTimeout(()=>resolve(v), 0));
function fetchTwice(x) {
    return _fetchTwice.apply(this, arguments);
}
function _fetchTwice() {
    _fetchTwice = _async_to_generator(function*(x) {
        log.push("start " + x);
        const a = yield later(x);
        const b = yield later(a + 1);
        return a + b;
    });
    return _fetchTwice.apply(this, arguments);
}
function guarded() {
    return _guarded.apply(this, arguments);
}
function _guarded() {
    _guarded = _async_to_generator(function*() {
        try {
            yield later(0);
            throw new TypeError("t");
        } catch (e) {
            log.push("caught " + e.name);
            return "r";
        } finally{
            log.push("finally");
        }
    });
    return _guarded.apply(this, arguments);
}
const service = {
    base: 100,
    total (n) {
        return _async_to_generator(function*() {
            var _this = this, _arguments = arguments;
            const add = /*#__PURE__*/ function() {
                var _ref = _async_to_generator(function*(k) {
                    return _this.base + k + _arguments.length;
                });
                return function add(k) {
                    return _ref.apply(this, arguments);
                };
            }();
            let sum = 0;
            for(let i = 0; i < n; i++)sum += yield add(i);
            return sum;
        }).apply(this);
    }
};
class Loader {
    load(v) {
        var _this = this;
        return _async_to_generator(function*() {
            yield null;
            _this.items.push(v);
            return _this.items.length;
        })();
    }
    constructor(){
        this.items = [];
    }
}
function main() {
    return _main.apply(this, arguments);
}
function _main() {
    _main = _async_to_generator(function*() {
        const p = fetchTwice(1);
        log.push("after call");
        console.log((yield p), (yield guarded()), (yield service.total(3)));
        const l = new Loader();
        console.log((yield l.load("a")), (yield l.load("b")));
        const arrow = /*#__PURE__*/ function() {
            var _ref = _async_to_generator(function*(n) {
                return (yield later(n)) * 2;
            });
            return function arrow(n) {
                return _ref.apply(this, arguments);
            };
        }();
        console.log((yield arrow(4)), (yield Promise.all([
            arrow(1),
            fetchTwice(2)
        ])).join(","));
        console.log(log.join("; "));
    });
    return _main.apply(this, arguments);
}
main().catch((e)=>console.log("fail", e));
