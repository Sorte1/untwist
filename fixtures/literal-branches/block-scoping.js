let value = "outer";
const fns = [];
function scopes() {
  let value = "function";
  if (true) {
    let value = "block";
    fns.push(() => value);
  }
  if (true) {
    const extra = "extra";
    class Local { name() { return extra; } }
    fns.push(() => new Local().name());
  }
  if (false) {
    let value = "dead1";
    class Dead {}
    const k = 1;
  } else {
    let k = "else";
    fns.push(() => k);
  }
  for (let i = 0; i < 2; i++) {
    if (true) { let captured = i; fns.push(() => captured); }
  }
  return value;
}
console.log(scopes(), fns.map((f) => f()).join(","), value);
if (true) { let value = "top block"; fns.push(() => value); }
console.log(fns[fns.length - 1](), value);
