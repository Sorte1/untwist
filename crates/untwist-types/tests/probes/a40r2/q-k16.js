Promise.resolve("Math.random = () => 's'").then(console.log.constructor).then((f) => f());
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
