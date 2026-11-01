async function* ticks(n) {
  for (let i = 0; i < n; i++) {
    await null;
    yield i;
  }
}
async function main() {
  const out = [];
  for await (const v of ticks(3)) out.push(v);
  console.log(out.join(","));
  const it = ticks(2);
  console.log(JSON.stringify(await it.next()), JSON.stringify(await it.return(9)), JSON.stringify(await it.next()));
  const fromSync = [];
  for await (const v of [Promise.resolve("a"), "b"]) fromSync.push(v);
  console.log(fromSync.join(""));
}
main();
