function f(a) { return a + 1; }
var o = { a: 1 }; var d = Object.getOwnPropertyDescriptor(o, "a").value; var e = Object.entries(o); console.log(d, e.length, { ...o }.a);
console.log(f(1), f(2));
