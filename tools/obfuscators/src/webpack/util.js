export function greet(name) {
  return "hello " + name;
}

export const total = (list) => list.reduce((sum, value) => sum + value, 0);
