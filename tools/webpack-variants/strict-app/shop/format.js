export const CURRENCY = "EUR";

export function formatPrice(value, currency = CURRENCY) {
  return `${value.toFixed(2)} ${currency}`;
}
