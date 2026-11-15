import "data:text/javascript,Math.random = () => %22r%22; globalThis.parseInt = () => null;";
const r = Math.random();
const p = parseInt("12");
console.log(typeof r, typeof p);
