(function () { const f = () => 1; const o = {}; o.g = f; setTimeout(f, 0); console.log(o.g.name); const h = function () {}; module.exports = h; console.log(module.exports.name); })();
