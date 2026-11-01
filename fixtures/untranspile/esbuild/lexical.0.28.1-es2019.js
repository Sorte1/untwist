const fns = [];
for (let i = 0; i < 3; i++) fns.push(() => i);
console.log(fns.map((f) => f()).join(","));
const fns2 = [];
for (const k in { a: 1, b: 2 }) fns2.push(() => k);
console.log(fns2.map((f) => f()).join(","));
{
  let shadow = "inner";
  console.log(shadow);
}
const obj = {
  name: "o",
  arrow() {
    return [1].map(() => this.name);
  },
  plain() {
    return [1].map(function() {
      return typeof this;
    });
  }
};
console.log(obj.arrow()[0], obj.plain()[0]);
function argsArrow() {
  const a2 = () => arguments.length;
  return a2(1, 2, 3);
}
console.log(argsArrow(7));
const tpl = (x) => `v=${x} ${typeof x}`;
console.log(tpl(1), tpl.name, tpl.length);
const def = (a2, b = a2 + 1, ...c) => [a2, b, c.length];
console.log(def(1).join(","), def.length);
try {
  let tdz2 = function() {
    return later;
  };
  var tdz = tdz2;
  tdz2();
  let later = 1;
} catch (e) {
  console.log(e.constructor.name);
}
var hoisted = typeof laterFn;
function laterFn() {
}
console.log(hoisted);
console.log(typeof (() => {
}).prototype, typeof function() {
}.prototype);
let closures = [];
let j = 0;
while (j < 2) {
  let captured = j;
  closures.push(() => captured);
  j++;
}
console.log(closures.map((f) => f()).join(","));
const short = { a, b() {
  return "b";
} };
function a() {
  return "a";
}
console.log(short.a(), short.b(), typeof short.b.prototype);
