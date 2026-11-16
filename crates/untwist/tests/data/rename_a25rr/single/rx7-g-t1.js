function f(a) { return a + 1; }
var self = this; var R = self.Reflect; R.get(R, "set")(self, "f", function () { return 99; });
console.log(f(1), f(2));
