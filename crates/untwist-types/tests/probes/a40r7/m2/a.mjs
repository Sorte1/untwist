export const str = { [Symbol.toPrimitive]() { return 1; } };
export function mk() { return str; }
