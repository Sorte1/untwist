"use strict";
async function one(x) {
    return x;
}
console.log(one.length, Object.getPrototypeOf(one) === Function.prototype, typeof one.prototype);
const order = [];
async function a1() {
    order.push(1);
    await undefined;
    order.push(4);
    await undefined;
    order.push(6);
}
a1();
order.push(2);
Promise.resolve()
    .then(() => order.push(5))
    .then(() => order.push(7))
    .then(() => order.push(8));
order.push(3);
setTimeout(() => console.log(order.join(",")), 0);
async function thenable() {
    return { then(resolve) { resolve("thenable"); } };
}
thenable().then((v) => console.log(v));
