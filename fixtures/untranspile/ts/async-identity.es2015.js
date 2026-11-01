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
function one(x) {
    return __awaiter(this, void 0, void 0, function* () {
        return x;
    });
}
console.log(one.length, Object.getPrototypeOf(one) === Function.prototype, typeof one.prototype);
const order = [];
function a1() {
    return __awaiter(this, void 0, void 0, function* () {
        order.push(1);
        yield undefined;
        order.push(4);
        yield undefined;
        order.push(6);
    });
}
a1();
order.push(2);
Promise.resolve()
    .then(() => order.push(5))
    .then(() => order.push(7))
    .then(() => order.push(8));
order.push(3);
setTimeout(() => console.log(order.join(",")), 0);
function thenable() {
    return __awaiter(this, void 0, void 0, function* () {
        return { then(resolve) { resolve("thenable"); } };
    });
}
thenable().then((v) => console.log(v));
