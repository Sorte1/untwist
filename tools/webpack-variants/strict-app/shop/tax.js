export const RATE = 0.2;

export function withTax(value) {
  return Math.round(value * (1 + RATE) * 100) / 100;
}
