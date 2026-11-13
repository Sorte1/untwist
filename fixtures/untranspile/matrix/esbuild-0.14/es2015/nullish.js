var _a, _b, _c, _d, _e;
function pick(a, b) {
  return a != null ? a : b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(void 0, "u"), pick(false, true));
const cfg = { retries: 0, name: "" };
const retries = (_a = cfg.retries) != null ? _a : 3;
const name = (_b = cfg.name) != null ? _b : "anon";
const missing = (_c = cfg.timeout) != null ? _c : 1e3;
console.log(retries, name, missing);
let calls = 0;
const side = () => {
  calls++;
  return null;
};
console.log((_e = (_d = side()) != null ? _d : side()) != null ? _e : "end", calls);
console.log("or", "inner");
