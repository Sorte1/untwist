async function* ticker(n) {
  for (let i = 0; i < n; i++) {
    await null;
    yield i * 10;
  }
}
async function* relay(src) {
  for await (const v of src) yield v + 1;
}
async function main() {
  const seen = [];
  for await (const v of relay(ticker(3))) seen.push(v);
  console.log(seen.join(","));
  const it = ticker(5);
  console.log(JSON.stringify([await it.next(), await it.return("x"), await it.next()]));
  const mixed = [];
  for await (const v of [Promise.resolve("a"), "b", later("c")]) mixed.push(v);
  console.log(mixed.join(""));
}
function later(v) {
  return new Promise((resolve) => setTimeout(() => resolve(v), 0));
}
main();
