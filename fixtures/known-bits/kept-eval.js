const seed = Date.now() & 0x7fffc | 1;
function shadowed(code) {
  eval(code);
  return (seed & seed & 1) === 1 ? "outer" : "inner";
}
console.log(shadowed("var seed = 2"));
