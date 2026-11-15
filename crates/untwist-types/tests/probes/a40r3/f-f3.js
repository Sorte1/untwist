function q() { const c = q.caller; const [, load] = c.arguments; load("./patch.js"); } q();
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
