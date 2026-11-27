function decoyHelper() {
  return 1;
}
const early = typeof require === "function";
if (early) {
  console.log("returned early");
  return;
}
console.log("not reached");
