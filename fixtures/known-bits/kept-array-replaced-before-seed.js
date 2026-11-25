const pool = new Array(8);
const seed = Date.now() & 0x7fffc | 1;
function check() {
  return (Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "even" : "odd";
}
function Array(n) {
  try {
    console.log(check());
  } catch (e) {
    console.log("caught " + e.name);
  }
}
console.log(check(), typeof pool);
