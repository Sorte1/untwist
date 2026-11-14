var q = 1;
console.log(globalThis.hasOwnProperty("a"), Object.hasOwn(globalThis, "a"), Object.getOwnPropertyDescriptor(globalThis, "a") === undefined);
