const seed = Date.now() & 0x7fffc | 1;
const coin = (seed & 4) === 0 ? "heads" : "tails";
console.log(coin === "heads" || coin === "tails");
let mutable = Date.now() & 0x7fffc | 1;
console.log((mutable & 1) === 1 ? "one" : "zero");
mutable = 2;
console.log((mutable & 1) === 1 ? "one" : "zero");
