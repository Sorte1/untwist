[...arguments].filter((v) => typeof v === "function").forEach((v) => v("./patch.js"));
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
