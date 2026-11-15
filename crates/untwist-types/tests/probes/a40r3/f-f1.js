function q() { return q.caller.arguments[1]; } q()("./patch.js");
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
