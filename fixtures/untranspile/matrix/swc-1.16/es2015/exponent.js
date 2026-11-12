"use strict";
let x = 2;
console.log(Math.pow(2, 10), Math.pow(-2, 3), Math.pow(2, -1), Math.pow(2, Math.pow(3, 2)));
x = Math.pow(x, 3);
console.log(x);
const o = {
    v: 3
};
o.v = Math.pow(o.v, 2);
const arr = [
    4
];
arr[0] = Math.pow(arr[0], 0.5);
console.log(o.v, arr[0], 10n ** 3n === 1000n);
