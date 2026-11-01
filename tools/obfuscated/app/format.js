export function pad(text, width, right = false) {
  const fill = " ".repeat(Math.max(0, width - text.length));
  return right ? fill + text : text + fill;
}

export function table(record) {
  return Object.entries(record)
    .map(([key, value]) => pad(key, 8) + "= " + String(value))
    .join("\n");
}
