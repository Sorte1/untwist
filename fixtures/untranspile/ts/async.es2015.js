"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
const log = [];
const sleep = (v) => new Promise((resolve) => setTimeout(() => resolve(v), 0));
function one(x) {
    return __awaiter(this, void 0, void 0, function* () {
        log.push("one-start " + x);
        const y = yield x;
        log.push("one-after " + y);
        return y * 2;
    });
}
function throws() {
    return __awaiter(this, void 0, void 0, function* () {
        yield null;
        throw new RangeError("r");
    });
}
function withFinally() {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            yield throws();
        }
        catch (e) {
            log.push("caught " + e.constructor.name);
            return "ret";
        }
        finally {
            log.push("finally");
        }
    });
}
const obj = {
    base: 10,
    m(a) {
        var arguments_1 = arguments;
        return __awaiter(this, void 0, void 0, function* () {
            const f = () => __awaiter(this, void 0, void 0, function* () { return this.base + a + arguments_1.length; });
            return yield f();
        });
    },
};
function loop() {
    return __awaiter(this, void 0, void 0, function* () {
        let s = 0;
        for (let i = 0; i < 3; i++) {
            s += yield sleep(i);
        }
        return s;
    });
}
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        const p = one(1);
        log.push("sync after call");
        console.log(yield p, yield withFinally(), yield obj.m(5), yield loop());
        console.log(one.name, typeof one);
        console.log(log.join("; "));
        const arrow = (n) => __awaiter(this, void 0, void 0, function* () { return (yield sleep(n)) + 1; });
        console.log(yield arrow(1), yield Promise.all([one(2), arrow(3)]).then((v) => v.join(",")));
    });
}
main().catch((e) => console.log("fail", e.constructor.name));
