if (typeof 0[globalThis.atob = function () { return "patched"; }] === "x") {}
console.log(atob("aGk="));
