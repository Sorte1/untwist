const seen = [];
outer: for (let i = 0; i < 3; i++) {
  inner: if (false) {
    seen.push("dead1");
    break inner;
  }
  block: if (true) {
    seen.push("block" + i);
    if (i === 1) break block;
    seen.push("after" + i);
  }
  if (true) {
    if (i === 1) continue outer;
    seen.push("second" + i);
  }
  if (true) {
    if (i === 2) break outer;
  }
  while (false) { continue outer; }
  seen.push("end" + i);
}
lone: while (false) { seen.push("dead2"); break lone; }
lone2: if (0) seen.push("dead3");
console.log(seen.join(","));
