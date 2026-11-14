var o = { width: Math.random() < 2 ? 1 : 0 };
var q = o.width;
console.log(q, "width" in globalThis);
