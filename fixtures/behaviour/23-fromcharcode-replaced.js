function a() { var h = ['w', 'x', 'y', 'z']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
return String.fromCharCode(e[f].charCodeAt(0));
}), b(c, d);
}
String.fromCharCode = function () { return 'X'; };
console.log(b(0));
console.log(b(1));
console.log(b(2));
console.log(b(3));
