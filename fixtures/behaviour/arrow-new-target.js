function Outer() {
  var plain = function () {
    return new.target === undefined;
  };
  this.result = plain();
}
console.log(new Outer().result);
