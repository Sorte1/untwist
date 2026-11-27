function decoyA() {
  return 3;
}
function decoyB() {
  return 4;
}
const decoyC = [5];
module.exports = { decoyA };
module.exports.b = decoyB;
exports.c = decoyC;
console.log(Object.keys(module.exports).join(","), module.exports.decoyA(), module.exports.b());
