var q = 1; function zz() { return 2; }
var t = Function("return setTimeout")();
t("console.log(typeof q + typeof zz)", 0);
