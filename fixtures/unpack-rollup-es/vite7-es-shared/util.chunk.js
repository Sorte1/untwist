console.log("eval util");
const label = (name) => `[${name.padStart(8, ".")}]`;
function total(list) {
  return list.reduce((sum, n) => sum + n, 0);
}
class Counter {
  constructor() {
    this.value = 0;
  }
  add(n) {
    this.value += n;
  }
}
export {
  Counter as C,
  label as l,
  total as t
};
