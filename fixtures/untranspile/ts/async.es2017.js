"use strict";
const log = [];
const sleep = (v) => new Promise((resolve) => setTimeout(() => resolve(v), 0));
async function one(x) {
    log.push("one-start " + x);
    const y = await x;
    log.push("one-after " + y);
    return y * 2;
}
async function throws() {
    await null;
    throw new RangeError("r");
}
async function withFinally() {
    try {
        await throws();
    }
    catch (e) {
        log.push("caught " + e.constructor.name);
        return "ret";
    }
    finally {
        log.push("finally");
    }
}
const obj = {
    base: 10,
    async m(a) {
        const f = async () => this.base + a + arguments.length;
        return await f();
    },
};
async function loop() {
    let s = 0;
    for (let i = 0; i < 3; i++) {
        s += await sleep(i);
    }
    return s;
}
async function main() {
    const p = one(1);
    log.push("sync after call");
    console.log(await p, await withFinally(), await obj.m(5), await loop());
    console.log(one.name, typeof one);
    console.log(log.join("; "));
    const arrow = async (n) => (await sleep(n)) + 1;
    console.log(await arrow(1), await Promise.all([one(2), arrow(3)]).then((v) => v.join(",")));
}
main().catch((e) => console.log("fail", e.constructor.name));
