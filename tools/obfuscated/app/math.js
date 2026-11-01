export const PRECISION = 3;

export function add(a, b) {
  return a + b;
}

export function scale(value, factor = 1.5) {
  return Number((value * factor).toFixed(PRECISION));
}
