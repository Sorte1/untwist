function computeSessionDigest_a1b(key) {
  return key + 1;
}
const sealPrimaryLedger_c2d = 7;
function decodeLedger_e3f(x) {
  return x * 2;
}
module.exports = { tryUnlock: computeSessionDigest_a1b, decode: decodeLedger_e3f };
module.exports.SCOPE = sealPrimaryLedger_c2d;
console.log(Object.keys(module.exports).join(","), module.exports.tryUnlock(sealPrimaryLedger_c2d), decodeLedger_e3f(4));
