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
const log = [];
const later = (v)=>new Promise((resolve)=>setTimeout(()=>resolve(v), 0));
function fetchTwice(x) {
    return _async_to_generator(function*() {
        log.push("start " + x);
        const a = yield later(x);
        const b = yield later(a + 1);
        return a + b;
    })();
}
function guarded() {
    return _async_to_generator(function*() {
        try {
            yield later(0);
            throw new TypeError("t");
        } catch (e) {
            log.push("caught " + e.name);
            return "r";
        } finally{
            log.push("finally");
        }
    })();
}
const service = {
    base: 100,
    total (_0) {
        return _async_to_generator(function*(n) {
            const add = (k)=>_async_to_generator(function*() {
                    return this.base + k + arguments.length;
                }).apply(this, arguments);
            let sum = 0;
            for(let i = 0; i < n; i++)sum += yield add(i);
            return sum;
        }).apply(this, arguments);
    }
};
class Loader {
    load(v) {
        return _async_to_generator(function*() {
            yield null;
            this.items.push(v);
            return this.items.length;
        }).call(this);
    }
    constructor(){
        this.items = [];
    }
}
function main() {
    return _async_to_generator(function*() {
        const p = fetchTwice(1);
        log.push("after call");
        console.log((yield p), (yield guarded()), (yield service.total(3)));
        const l = new Loader();
        console.log((yield l.load("a")), (yield l.load("b")));
        const arrow = (n)=>_async_to_generator(function*() {
                return (yield later(n)) * 2;
            })();
        console.log((yield arrow(4)), (yield Promise.all([
            arrow(1),
            fetchTwice(2)
        ])).join(","));
        console.log(log.join("; "));
    })();
}
main().catch((e)=>console.log("fail", e));
