const k = Object.keys({ constructor: 1 })[0];
const F = ({ __proto__: Object })[k];
F("Promise.prototype.then = function () { return 5; }")();
(async function () { const v = await Promise.resolve(1); console.log(typeof v); })();
async function h() { return 1; }
const pr = h();
const t = pr.then(function (x) { return x; });
console.log(typeof t);
