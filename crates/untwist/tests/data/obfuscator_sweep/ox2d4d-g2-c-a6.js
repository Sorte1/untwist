function f() { const holder = 1; var arr0 = ["two", "three", "one"]; function r(e) { for (; --e;) arr0.push(arr0.shift()); } (holder = { updateCookie: function () { return true; } }).updateCookie() ? r(3) : r(2); return arr0.join(); }
try { console.log(f()); } catch (e) { console.log("ERR", e.constructor.name); }
