const decoyEffect = [console.log("element ran")];
const decoyKey = { [(console.log("key ran"), "k")]: 1 };
const decoyGetter = { get x() { console.log("getter ran"); return 1; } };
const decoyProto = { __proto__: { inherited: true } };
const decoySpread = [...[1, 2]];
const decoyCall = Object.freeze([String(1)]);
console.log("done");
