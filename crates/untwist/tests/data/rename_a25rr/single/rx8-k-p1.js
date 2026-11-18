function f(a) { return a + 1; }
var o = { a: 1, b: 2 }; var t = 0; for (var k in o) t += o[k]; var q = []; for (var j in o) q.push(function (i) { return o[i]; }.bind(null, j)); Object.keys(o).forEach(function (m) { t += o[m]; }); console.log(t, q[0](), o[Symbol.iterator]);
console.log(f(1), f(2));
