// tools/rolldown/app/lazy.js
console.log("eval lazy");
const value = 42;
var lazy_default = "lazy-default";

export { lazy_default as default, value };