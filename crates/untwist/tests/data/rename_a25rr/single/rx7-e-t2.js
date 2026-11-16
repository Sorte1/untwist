function f(a) { return a + 1; }
(function () { let g; try { const F = Function("return this"); g = F(); } catch (e) { g = globalThis; } g.setTimeout(function () { f = function () { return 99; }; }, 0); })();
setTimeout(function () { console.log(f(1), f(2)); }, 5);
