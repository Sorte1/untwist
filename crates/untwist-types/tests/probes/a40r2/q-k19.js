String({ toString() { Math.random = () => "s"; return "x"; } });
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
