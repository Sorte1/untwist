export function greet(name) {
  return "hello " + name;
}

export const total = (list) => list.reduce((sum, value) => sum + value, 0);

let made = 0;

export class Counter {
  constructor(start) {
    made += 1;
    this.value = start;
  }
  add(step) {
    this.value += step;
    return this;
  }
}

export function makeAdders(count) {
  const adders = [];
  for (let index = 0; index < count; index++) {
    adders.push((value) => value + index + made);
  }
  return adders;
}
