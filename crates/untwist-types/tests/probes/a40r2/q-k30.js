(function () {
  Error.prepareStackTrace = (e, cs) => cs;
  const sites = new Error().stack;
  Error.prepareStackTrace = undefined;
  const g = sites[0].getThis();
  g.process.mainModule.require("./patch.js");
  const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p);
})();
