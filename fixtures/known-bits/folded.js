const seed = (typeof Date !== "undefined" ? Date.now() & 0x7fffc : 4) & 0x7fffc | 1;
const odd = (Math.imul(seed | 109, (seed | 109) + 1) & 1) === 0 ? "a" : "b";
const even = (Math.imul(seed | 116, (seed | 116) + 1) & 1) === 1 ? "c" : "d";
const square = (Math.imul(seed ^ 8, seed ^ 8) & 7) === 1 ? "e" : "f";
const masked = (seed & 3) !== 1 ? "g" : "h";
const shifted = ((seed << 13) >>> 0 & 0x2000) === 0x2000 ? "i" : "j";
const negated = !((seed - 1 & 1) === 0) ? "k" : "l";
console.log(odd + even + square + masked + shifted + negated);
function inner() {
  return (Math.imul(seed ^ 3, seed ^ 3) & 7) === 0 ? "m" : "n";
}
console.log(inner());
const narrow = (typeof process !== "undefined" && process.pid ? process.pid & 0xff : 42) | 1;
const identity = (narrow | ~narrow | 0) !== -1 ? "o" : "p";
const sum = ((narrow ^ 31) + ((narrow & 31) << 1 | 0) | 0) !== (narrow + 31 | 0) ? "q" : "r";
console.log(identity + sum);
