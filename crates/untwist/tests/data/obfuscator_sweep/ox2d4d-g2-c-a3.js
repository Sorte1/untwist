function f() { var arr0 = ["two", "three", "one"]; var holder; al = arr0; function r(e) { for (; --e;) al.push(al.shift()); } (holder = { updateCookie: function () { return true; } }).updateCookie() ? r(3) : r(2); let al; return arr0.join(); }
try { console.log(f()); } catch (e) { console.log("ERR", e.constructor.name); }
