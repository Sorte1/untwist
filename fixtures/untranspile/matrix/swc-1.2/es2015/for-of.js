"use strict";
const out = [];
for (const v of [
    1,
    2,
    3
])out.push(v * 2);
for (const ch of "héllo")out.push(ch);
for (const [k, v1] of new Map([
    [
        "a",
        1
    ],
    [
        "b",
        2
    ]
]))out.push(k + v1);
for (const v2 of new Set([
    3,
    3,
    4
]))out.push(v2);
console.log(out.join(","));
const custom = {
    from: 1,
    to: 4,
    [Symbol.iterator] () {
        let cur = this.from;
        const last = this.to;
        return {
            next: ()=>cur <= last ? {
                    value: cur++,
                    done: false
                } : {
                    value: undefined,
                    done: true
                },
            return () {
                out.push("closed");
                return {
                    done: true
                };
            }
        };
    }
};
const got = [];
for (const v3 of custom){
    if (v3 === 3) break;
    got.push(v3);
}
console.log(got.join(","), out[out.length - 1]);
function args() {
    const xs = [];
    for (const a of arguments)xs.push(a);
    return xs.join("+");
}
console.log(args(1, 2, 3));
let total = 0;
outer: for (const row of [
    [
        1,
        2
    ],
    [
        3,
        4
    ],
    [
        5,
        6
    ]
]){
    for (const cell of row){
        if (cell === 4) continue outer;
        if (cell === 6) break outer;
        total += cell;
    }
}
console.log(total);
