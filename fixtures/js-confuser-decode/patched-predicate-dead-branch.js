function __p_d() {}
function patch() { Object.prototype.k1 = 1; }
if ("zz" in __p_d) { patch(); }
this.patch();
console.log("k1" in __p_d ? "patched" : "clean");
