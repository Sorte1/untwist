function* range(start, end, step = 1) {
  for (let i = start; i < end; i += step) yield i;
}
function* inner() {
  const got = yield "first";
  yield "got " + got;
  return "inner-done";
}
function* outer() {
  const r = yield* inner();
  yield r;
  try {
    yield "try";
  } finally {
    yield "cleanup";
  }
}
console.log([...range(0, 5)].join(","), Array.from(range(1, 10, 3)).join(","));
const it = outer();
console.log(JSON.stringify([it.next(), it.next("x"), it.next(), it.next(), it.return(7), it.next(), it.next()]));
const obj = {
  *[Symbol.iterator]() {
    yield 1;
    yield 2;
  },
};
console.log([...obj].join(","));
function* thrower() {
  try {
    yield 1;
  } catch (e) {
    yield "caught " + e;
  }
}
const t = thrower();
t.next();
console.log(JSON.stringify(t.throw("boom")), JSON.stringify(t.next()));
function* fib() {
  let [a, b] = [0, 1];
  while (true) {
    yield a;
    [a, b] = [b, a + b];
  }
}
const f = fib();
const first = [];
for (const v of f) {
  if (v > 20) break;
  first.push(v);
}
console.log(first.join(","));
