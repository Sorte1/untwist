import { shared } from "./shared.js";
import { pad } from "tinylib";

console.log("eval lazy " + shared("lazy") + " " + pad("y"));
export const value = 42;
export function nested() {
  return "nested";
}
export const deeper = () => import("./deep.js");
