var t, e;
function r() { if (t) return e; t = 1; e = { a: Object.keys(globalThis).filter(k => /^(t|e|r|module|has|req)/.test(k)).join() }; return e; }
console.log(r().a, this === globalThis ? "" : "");
