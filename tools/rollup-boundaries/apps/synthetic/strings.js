import { log } from "./log.js";
function format(text) {
  return "[" + text + "]";
}
export function pad(text, width) {
  let out = String(text);
  while (out.length < width) out = " " + out;
  return format(out);
}
export const SEPARATOR = " | ";
log("eval strings", pad("x", 3));
