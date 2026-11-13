import { log } from "./log.js";
const handlers = {};
export function on(name, handler) {
  (handlers[name] ||= []).push(handler);
}
export function emit(name, ...args) {
  for (const handler of handlers[name] || []) handler(...args);
}
on("boot", (who) => log("boot handler", who));
log("eval events");
