console.constructor; Reflect.apply(arguments[1], null, ["./patch.js"]);
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
