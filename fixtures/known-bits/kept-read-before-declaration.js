function early() {
  return (Math.imul(seed | 2, (seed | 2) + 1) & 1) === 0 ? "even" : "odd";
}
try {
  console.log(early());
} catch (error) {
  console.log(error.name);
}
const seed = Date.now() & 0x7fffc | 1;
console.log(early());
