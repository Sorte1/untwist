function decoyProbe(path) {
  const g = typeof globalThis !== "undefined" ? globalThis : {};
  return g.process?.dlopen?.({ exports: {} }, path);
}
function decoyChainA(n) {
  return n > 0 ? decoyChainB(n - 1) : decoyProbe("enclave.node");
}
function decoyChainB(n) {
  return decoyChainA(n) ^ 1;
}
void decoyChainA;
function live(n) {
  return n * 2;
}
console.log(live(21), typeof globalThis.decoyProbe, typeof globalThis.live);
module.exports = { live };
