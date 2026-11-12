var ms = require("ms");
var assign = require("object-assign");
console.log("ms " + ms(90000) + " " + ms("2h"));
console.log("assign " + JSON.stringify(assign({ a: 1 }, { b: 2 })));
function later() {
  return require("ms")(1000);
}
console.log("later " + later());
