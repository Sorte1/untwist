var q = 1; function zz() { return 2; }
console.log(q + zz(), Function("return this.a + '' + this.b")());
