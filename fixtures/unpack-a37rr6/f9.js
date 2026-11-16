class A { #x = 0; m() { this?.#x = 1; return this.#x; } }
console.log(new A().m());
