var g = (console.log("head"), function () {});
var s = "JSON.stringify(g.name)"; console.log(Function("return eval")()(s));
