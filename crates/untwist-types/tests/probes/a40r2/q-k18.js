JSON.stringify({ toJSON() { Math.random = () => "s"; return 1; } });
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
