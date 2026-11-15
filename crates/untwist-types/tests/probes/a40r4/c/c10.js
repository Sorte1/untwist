function q(arguments) { return arguments.caller.arguments[1]; }
function p() { return q(p); }
p()("./patch.js");
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
