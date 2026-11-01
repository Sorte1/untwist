import { say } from "./say.js";
say("eval greet");
export default function greet(name) {
  return "hello " + name;
}
export function shout(name) {
  return name.toUpperCase() + "!";
}
