function patch(v, k) { v[k].charAt = function () { return "q"; }; } patch("", "__proto__");
