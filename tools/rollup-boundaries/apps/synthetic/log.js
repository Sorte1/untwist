export const lines = [];
export function log(...parts) {
  lines.push(parts.join(" "));
  console.log(...parts);
}
log("eval log");
