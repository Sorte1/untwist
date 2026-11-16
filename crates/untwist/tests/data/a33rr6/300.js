(function () { var a = function () {}; var b = a; var o = { g: a }; var c = [a][0]; function r() { return a; } console.log(b.name, o.g.name, c.name, r().name, Error.prototype.toString.call(b)); })();
