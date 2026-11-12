"use strict";
const name = "world";
const n = 3;
console.log(`hello ${name}`, `${n} + ${n} = ${n + n}`, `nested ${`inner ${name}`}`);
console.log(`multi
line`, `${{
    toString () {
        return "obj";
    }
}}`, `${[
    1,
    2
]}`);
function tag(strings, ...values) {
    return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag`a${1}b${2}c`, tag`\n${"x"}\t`);
function same(s) {
    return s;
}
const first = ()=>same`x`;
console.log(first() === first(), Object.isFrozen(first()), String.raw`\u{41}${1}\n`);
const sym = {
    valueOf () {
        return 1;
    },
    toString () {
        return "str";
    }
};
console.log(`${sym}`, "" + sym);
