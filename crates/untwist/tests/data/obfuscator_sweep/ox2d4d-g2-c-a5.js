function f(holder) { var arr0 = ["two", "three", "one"]; function r(e) { for (; --e;) arr0.push(arr0.shift()); } (holder = { updateCookie: function () { return true; } }).updateCookie() ? r(3) : r(2); return typeof arguments[0] + arr0.join(); }
console.log(f(1));
