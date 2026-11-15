function q() { Object.setPrototypeOf(arguments, Object); const k = Object.keys({ constructor: 1 })[0]; return arguments[k]; }
q()("return process.mainModule.require('./patch.js')")();
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
