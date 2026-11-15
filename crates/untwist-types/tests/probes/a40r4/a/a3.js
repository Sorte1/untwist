const k = Object.keys({ constructor: 1 })[0];
const F = ({ __proto__: Object })[k];
F("Math.random = () => 'patched'")();
(function () { const r = Math.random(); console.log(typeof r); })();
