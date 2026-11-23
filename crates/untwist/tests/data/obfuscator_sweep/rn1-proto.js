function g() { let __proto__ = 0; if (true) { let __proto__ = 5; const o = {__proto__}; console.log(Object.getPrototypeOf(o) === Object.prototype, o.__proto__ === 5 || Object.keys(o).join()); } return __proto__; }
console.log(g());
