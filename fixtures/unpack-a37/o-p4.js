function Big(v) {
  this.words = [v];
}
function inspect() {
  return String(this.toString().indexOf("_0x6") >= 0);
}
Big.prototype.inspect = inspect;
function fn(v) {
  return v;
}
fn.__proto__ = Big.prototype;
var mapped = [255].map((item) => {
  return fn.inspect();
});
console.log(mapped.join());
