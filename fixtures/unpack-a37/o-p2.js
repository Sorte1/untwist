function Big(v) {
  this.words = [v];
}
function inspect() {
  return String(this.toString().indexOf("_0x6") >= 0);
}
var obj = { m: Big.prototype.inspect = inspect };
var mapped = [255].map((item) => {
  return obj.m.call(function (v) {
    return v;
  });
});
console.log(mapped.join());
