function decoyHelper() {
  return 1;
}
this.answer = 42;
var top = 1;
console.log(module.exports.answer, this === module.exports, typeof globalThis.answer, typeof globalThis.top, top);
