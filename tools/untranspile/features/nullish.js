function pick(a, b) {
  return a ?? b;
}
console.log(pick(0, 1), pick("", "x"), pick(null, "n"), pick(undefined, "u"), pick(false, true));
const cfg = { retries: 0, name: "" };
const retries = cfg.retries ?? 3;
const name = cfg.name ?? "anon";
const missing = cfg.timeout ?? 1000;
console.log(retries, name, missing);
let calls = 0;
const side = () => {
  calls++;
  return null;
};
console.log(side() ?? side() ?? "end", calls);
console.log((null ?? 0) || "or", null ?? (0 || "inner"));
