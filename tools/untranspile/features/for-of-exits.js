const log = [];
function counted(name, items) {
  return {
    [Symbol.iterator]() {
      let at = 0;
      log.push(name + ":open");
      return {
        next() {
          log.push(name + ":next");
          return at < items.length ? { value: items[at++], done: false } : { value: undefined, done: true };
        },
        return() {
          log.push(name + ":return");
          return { done: true };
        },
      };
    },
  };
}
for (const v of counted("all", [1, 2])) log.push("all=" + v);
for (const v of counted("brk", [1, 2, 3])) {
  if (v === 2) break;
  log.push("brk=" + v);
}
function early() {
  for (const v of counted("ret", [5, 6, 7])) {
    if (v === 6) return v;
  }
  return -1;
}
log.push("early=" + early());
try {
  for (const v of counted("thr", [1, 2])) {
    if (v === 2) throw new Error("boom " + v);
  }
} catch (err) {
  log.push("caught " + err.message);
}
let skipped = 0;
for (const v of counted("cont", [1, 2, 3])) {
  if (v === 2) continue;
  skipped += v;
}
log.push("cont=" + skipped);
outer: for (const a of counted("outer", [1, 2])) {
  for (const b of counted("inner", [10, 20])) {
    if (b === 20) continue outer;
    log.push(a + b);
  }
}
let last;
for (last of counted("assign", [7, 8])) log.push("seen " + last);
log.push("last=" + last);
const pairs = new Map([["x", 1], ["y", 2]]);
for (const [key, value] of pairs) log.push(key + value);
const fns = [];
for (const v of [1, 2, 3]) fns.push(() => v);
log.push(fns.map((f) => f()).join(""));
function* gen() {
  try {
    yield 1;
    yield 2;
  } finally {
    log.push("gen:finally");
  }
}
for (const v of gen()) {
  if (v === 1) break;
}
try {
  for (const v of counted("bad", [1])) {
    log.push(v.missing.field);
  }
} catch (err) {
  log.push(err.constructor.name);
}
const spreadSet = [...new Set([4, 4, 5])];
const [firstKey] = new Map([["k", 1]]).keys();
log.push(spreadSet.join("") + firstKey);
console.log(log.join("\n"));
