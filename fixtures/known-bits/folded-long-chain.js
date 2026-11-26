const seed = Date.now() & 0x7fffc | 1;
const table = [{ name: "a" }, { name: "b" }];
function walk(flag) {
  const a0 = {};
  const a1 = flag ? a0 : 0;
  const a2 = flag ? a1 : 0;
  const a3 = flag ? a2 : 0;
  const a4 = flag ? a3 : 0;
  const a5 = flag ? a4 : 0;
  const a6 = flag ? a5 : 0;
  const a7 = flag ? a6 : 0;
  const a8 = flag ? a7 : 0;
  const a9 = flag ? a8 : 0;
  const a10 = flag ? a9 : 0;
  const a11 = flag ? a10 : 0;
  const a12 = flag ? a11 : 0;
  const a13 = flag ? a12 : 0;
  const a14 = flag ? a13 : 0;
  const a15 = flag ? a14 : 0;
  const a16 = flag ? a15 : 0;
  const a17 = flag ? a16 : 0;
  const a18 = flag ? a17 : 0;
  const a19 = flag ? a18 : 0;
  const a20 = flag ? a19 : 0;
  const a21 = flag ? a20 : 0;
  const a22 = flag ? a21 : 0;
  const a23 = flag ? a22 : 0;
  const a24 = flag ? a23 : 0;
  const a25 = flag ? a24 : 0;
  const a26 = flag ? a25 : 0;
  const a27 = flag ? a26 : 0;
  const a28 = flag ? a27 : 0;
  const a29 = flag ? a28 : 0;
  const a30 = flag ? a29 : 0;
  let total = "";
  for (let i = 0; i < table.length; i++) {
    const entry = table[i];
    total += entry.name;
  }
  return total + typeof a30;
}
console.log(walk(false));
console.log((Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "even" : "odd");
