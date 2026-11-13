"use strict";
const fns = [];
for (const v of new Set([1, 2, 3])) {
    const doubled = v * 2;
    let note = "n" + v;
    fns.push(() => doubled + note);
    note += "!";
}
console.log(fns.map((f) => f()).join(","));
const seen = [];
for (const [key, value] of new Map([["a", 1], ["b", 2]])) {
    seen.push(() => key + value);
}
console.log(seen.map((f) => f()).join(","));
const later = [];
for (const word of "abc") {
    let count = 0;
    later.push(() => word + ++count);
}
console.log(later.map((f) => f() + f()).join(","));
