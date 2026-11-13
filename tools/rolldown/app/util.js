console.log("eval util");
export const label = (name) => `[${name.padStart(8, ".")}]`;
export function total(list) {
  return list.reduce((sum, n) => sum + n, 0);
}
export class Counter {
  constructor() {
    this.value = 0;
  }
  add(n) {
    this.value += n;
  }
}
