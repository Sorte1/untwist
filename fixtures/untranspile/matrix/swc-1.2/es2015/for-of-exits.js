"use strict";
const log = [];
function counted(name, items) {
    return {
        [Symbol.iterator] () {
            let at = 0;
            log.push(name + ":open");
            return {
                next () {
                    log.push(name + ":next");
                    return at < items.length ? {
                        value: items[at++],
                        done: false
                    } : {
                        value: undefined,
                        done: true
                    };
                },
                return () {
                    log.push(name + ":return");
                    return {
                        done: true
                    };
                }
            };
        }
    };
}
for (const v of counted("all", [
    1,
    2
]))log.push("all=" + v);
for (const v1 of counted("brk", [
    1,
    2,
    3
])){
    if (v1 === 2) break;
    log.push("brk=" + v1);
}
function early() {
    for (const v of counted("ret", [
        5,
        6,
        7
    ])){
        if (v === 6) return v;
    }
    return -1;
}
log.push("early=" + early());
try {
    for (const v2 of counted("thr", [
        1,
        2
    ])){
        if (v2 === 2) throw new Error("boom " + v2);
    }
} catch (err) {
    log.push("caught " + err.message);
}
let skipped = 0;
for (const v3 of counted("cont", [
    1,
    2,
    3
])){
    if (v3 === 2) continue;
    skipped += v3;
}
log.push("cont=" + skipped);
outer: for (const a of counted("outer", [
    1,
    2
])){
    for (const b of counted("inner", [
        10,
        20
    ])){
        if (b === 20) continue outer;
        log.push(a + b);
    }
}
let last;
for (last of counted("assign", [
    7,
    8
]))log.push("seen " + last);
log.push("last=" + last);
const pairs = new Map([
    [
        "x",
        1
    ],
    [
        "y",
        2
    ]
]);
for (const [key, value] of pairs)log.push(key + value);
const fns = [];
for (const v4 of [
    1,
    2,
    3
])fns.push(()=>v4);
log.push(fns.map((f)=>f()).join(""));
function* gen() {
    try {
        yield 1;
        yield 2;
    } finally{
        log.push("gen:finally");
    }
}
for (const v5 of gen()){
    if (v5 === 1) break;
}
try {
    for (const v6 of counted("bad", [
        1
    ])){
        log.push(v6.missing.field);
    }
} catch (err1) {
    log.push(err1.constructor.name);
}
const spreadSet = [
    ...new Set([
        4,
        4,
        5
    ])
];
const [firstKey] = new Map([
    [
        "k",
        1
    ]
]).keys();
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
