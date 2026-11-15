var q = 1; function zz() { return 2; }
class C { static a = typeof globalThis.a; get b() { return typeof globalThis.b; } } console.log(q + zz(), C.a, new C().b);
