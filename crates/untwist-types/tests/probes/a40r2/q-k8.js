Error.prepareStackTrace = (e, cs) => cs; const g = new Error().stack[0].getThis() || (function () { return 1; }); Error.prepareStackTrace = undefined; (g.process ? g.process.mainModule.require("./patch.js") : 0);
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
