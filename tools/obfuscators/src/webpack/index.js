import { greet, total } from "./util.js";
import data from "./data.json";

console.log(greet("webpack"), total([1, 2, 3, 4]), data.name, data.items.length);
