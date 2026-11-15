async function z() {} ; Object.getPrototypeOf(z).constructor("Math.random = () => 's'")();
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
