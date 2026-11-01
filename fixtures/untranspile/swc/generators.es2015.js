function* counter(n) {
    try {
        for(let i = 0; i < n; i++){
            const got = yield i;
            if (got) console.log("got", got);
        }
        return "done";
    } finally{
        console.log("cleanup");
    }
}
const it = counter(3);
console.log(JSON.stringify(it.next()), JSON.stringify(it.next("a")));
console.log(JSON.stringify(it.return("early")), JSON.stringify(it.next()));
console.log([
    ...counter(2)
].join(","));
function* deleg() {
    const r = yield* counter(1);
    yield "r=" + r;
}
console.log([
    ...deleg()
].join(","));
function* thrower() {
    try {
        yield 1;
    } catch (e) {
        yield "caught " + e;
    }
    yield 3;
}
const t = thrower();
t.next();
console.log(t.throw("x").value, t.next().value, t.next().done);
function* labelled() {
    outer: for (const a of [
        1,
        2
    ]){
        for (const b of [
            1,
            2
        ]){
            if (b === 2) continue outer;
            yield a * 10 + b;
        }
    }
}
console.log([
    ...labelled()
].join(","));
const gen = {
    *[Symbol.iterator] () {
        yield "x";
        yield "y";
    }
};
console.log([
    ...gen
].join(""));
console.log(typeof counter, counter.name);
try {
    const g2 = counter(1);
    g2.next();
    g2.throw(new TypeError("t"));
} catch (e) {
    console.log(e.constructor.name);
}
function* fib() {
    let [x, y] = [
        0,
        1
    ];
    while(true){
        yield x;
        [x, y] = [
            y,
            x + y
        ];
    }
}
const firsts = [];
for (const v of fib()){
    if (v > 20) break;
    firsts.push(v);
}
console.log(firsts.join(","));
