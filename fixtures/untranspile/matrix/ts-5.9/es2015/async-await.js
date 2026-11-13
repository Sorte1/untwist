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
const later = (v) => new Promise((resolve) => setTimeout(() => resolve(v), 0));
function fetchTwice(x) {
    return __awaiter(this, void 0, void 0, function* () {
        log.push("start " + x);
        const a = yield later(x);
        const b = yield later(a + 1);
        return a + b;
    });
}
function guarded() {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            yield later(0);
            throw new TypeError("t");
        }
        catch (e) {
            log.push("caught " + e.name);
            return "r";
        }
        finally {
            log.push("finally");
        }
    });
}
const service = {
    base: 100,
    total(n) {
        var arguments_1 = arguments;
        return __awaiter(this, void 0, void 0, function* () {
            const add = (k) => __awaiter(this, void 0, void 0, function* () { return this.base + k + arguments_1.length; });
            let sum = 0;
            for (let i = 0; i < n; i++)
                sum += yield add(i);
            return sum;
        });
    },
};
class Loader {
    constructor() {
        this.items = [];
    }
    load(v) {
        return __awaiter(this, void 0, void 0, function* () {
            yield null;
            this.items.push(v);
            return this.items.length;
        });
    }
}
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        const p = fetchTwice(1);
        log.push("after call");
        console.log(yield p, yield guarded(), yield service.total(3));
        const l = new Loader();
        console.log(yield l.load("a"), yield l.load("b"));
        const arrow = (n) => __awaiter(this, void 0, void 0, function* () { return (yield later(n)) * 2; });
        console.log(yield arrow(4), (yield Promise.all([arrow(1), fetchTwice(2)])).join(","));
        console.log(log.join("; "));
    });
}
main().catch((e) => console.log("fail", e));
