var q = 1; function zz() { return 2; }
console.log(q + zz(), (function () { return typeof this.a + typeof this.b; })());
