class A extends Object { static make() { return super.constructor; } } A.make()("Math.random = () => 's'")();
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
