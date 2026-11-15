class P { static get exports() { return this; } }
Object.defineProperty(Object.prototype, "exports", Object.getOwnPropertyDescriptors(P).exports);
