const C = console.log.constructor; C("Math.random = () => 's'")();
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
