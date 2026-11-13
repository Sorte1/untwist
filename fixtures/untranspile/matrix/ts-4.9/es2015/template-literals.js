"use strict";
var __makeTemplateObject = (this && this.__makeTemplateObject) || function (cooked, raw) {
    if (Object.defineProperty) { Object.defineProperty(cooked, "raw", { value: raw }); } else { cooked.raw = raw; }
    return cooked;
};
const name = "world";
const n = 3;
console.log(`hello ${name}`, `${n} + ${n} = ${n + n}`, `nested ${`inner ${name}`}`);
console.log(`multi
line`, `${{ toString() { return "obj"; } }}`, `${[1, 2]}`);
function tag(strings, ...values) {
    return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag `a${1}b${2}c`, tag `\n${"x"}\t`);
function same(s) {
    return s;
}
const first = () => same `x`;
console.log(first() === first(), Object.isFrozen(first()), String.raw(__makeTemplateObject([void 0, "\n"], ["\\u{41}", "\\n"]), 1));
const sym = { valueOf() { return 1; }, toString() { return "str"; } };
console.log(`${sym}`, "" + sym);
