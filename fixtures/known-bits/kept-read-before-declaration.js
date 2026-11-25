function early() {
  return (seed & 1) === 1 ? "one" : "zero";
}
try {
  console.log(early());
} catch (error) {
  console.log(error.name);
}
const seed = Date.now() & 0x7fffc | 1;
console.log(early());
