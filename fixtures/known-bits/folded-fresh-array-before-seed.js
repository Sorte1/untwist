const pool = new Array(8);
const empty = new Array();
const seed = Date.now() & 0x7fffc | 1;
function pick() {
  return (Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "even" : "odd";
}
pool[0] = pick();
console.log(pool[0], empty.length, pool.length);
