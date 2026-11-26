const seed = Date.now() & 0x7fffc | 1;
(function () {
  const root = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : {};
  if (root && typeof Object.defineProperty === "function" && !root["cacheSlot"]) {
    Object.defineProperty(root, "cacheSlot", {
      configurable: true,
      get: function () {
        return 7;
      }
    });
  }
  root.manifest = { name: "x" };
  console.log(typeof root.process, root.cacheSlot, root.manifest.name);
})();
console.log((Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "even" : "odd");
