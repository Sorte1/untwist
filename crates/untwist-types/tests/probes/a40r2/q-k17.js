const s = Symbol.for("nodejs.util.inspect.custom"); const o = { [s]() { Math.random = () => "s"; return "x"; } }; console.log(o);
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
