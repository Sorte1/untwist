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
const sleep = (v)=>new Promise((resolve)=>setTimeout(()=>resolve(v), 0));
function one(x) {
    return _async_to_generator(function*() {
        log.push("one-start " + x);
        const y = yield x;
        log.push("one-after " + y);
        return y * 2;
    })();
}
function throws() {
    return _async_to_generator(function*() {
        yield null;
        throw new RangeError("r");
    })();
}
function withFinally() {
    return _async_to_generator(function*() {
        try {
            yield throws();
        } catch (e) {
            log.push("caught " + e.constructor.name);
            return "ret";
        } finally{
            log.push("finally");
        }
    })();
}
const obj = {
    base: 10,
    m (_0) {
        return _async_to_generator(function*(a) {
            const f = ()=>_async_to_generator(function*() {
                    return this.base + a + arguments.length;
                }).apply(this, arguments);
            return yield f();
        }).apply(this, arguments);
    }
};
function loop() {
    return _async_to_generator(function*() {
        let s = 0;
        for(let i = 0; i < 3; i++){
            s += yield sleep(i);
        }
        return s;
    })();
}
function main() {
    return _async_to_generator(function*() {
        const p = one(1);
        log.push("sync after call");
        console.log((yield p), (yield withFinally()), (yield obj.m(5)), (yield loop()));
        console.log(one.name, typeof one);
        console.log(log.join("; "));
        const arrow = (n)=>_async_to_generator(function*() {
                return (yield sleep(n)) + 1;
            })();
        console.log((yield arrow(1)), (yield Promise.all([
            one(2),
            arrow(3)
        ]).then((v)=>v.join(","))));
    })();
}
main().catch((e)=>console.log("fail", e.constructor.name));
