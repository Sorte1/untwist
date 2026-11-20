function patch(v, k) { v[k].prototype.charAt = function () { return "q"; }; } patch("", "constructor");
