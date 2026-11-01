var n = 0;
function a() { var h = ['log', 'deobfuscate', 'console', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c) { var e = a(); n++; return e[(c + n) % 4]; }
console.log(b(0));
console.log(b(0));
console.log(b(1));
