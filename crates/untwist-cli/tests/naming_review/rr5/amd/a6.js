define(["path"], function (p) { "use strict"; p = function () {}; var q = p; return { p: q, s: Error.prototype.toString.call(p) }; });
