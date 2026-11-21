function f() { const al = 0; var arr0 = ["two", "three", "one"]; var holder; al = arr0; function r(e) { for (; --e;) al.push(al.shift()); } (holder = { updateCookie: function () { return true; } }).updateCookie() ? r(3) : r(2); return arr0.join(); }
try { console.log(f()); } catch (e) { console.log("ERR", e.constructor.name); }
