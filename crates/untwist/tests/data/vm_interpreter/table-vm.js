var program = "QUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJD";
var handlers = [];
for (var k = 0; k < 64; k++) handlers.push(function (state) { state.sum += 1; });
function Machine(text) { this.text = text; this.ip = 0; this.sum = 0; }
Machine.prototype.readOp = function () { var c = this.text.charCodeAt(this.ip++); return c === c ? c % 64 : undefined; };
Machine.prototype.run = function () {
  for (;;) {
    var op = this.readOp();
    if (op === undefined) break;
    handlers[op](this);
  }
  return this.sum;
};
console.log(new Machine(program).run());
