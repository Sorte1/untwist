console.log("eval util");
const e = (t) => `[${t.padStart(8, ".")}]`;
function o(t) {
  return t.reduce((a, l) => a + l, 0);
}
class s {
  constructor() {
    this.value = 0;
  }
  add(a) {
    this.value += a;
  }
}
export {
  s as C,
  e as l,
  o as t
};
