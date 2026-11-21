function obfcegan(humon, keban) {
  humon = humon - 366;
  const penan = obfciden();
  const filen = penan[humon];
  return filen;
}
(function () {})();
function obfciden() {
  const cafen = ["prototype", "function *\\( *\\)", "chain", "warn", "{}.constructor(\"return this\")( )", "action", "log", "search", "return (function() ", "setInterval", "table", "bind", "console", "constructor", "apply", "length", "trace", "test", "toString", "(((.+)+)+)+$", "counter", "gger", "string"];
  obfciden = function () {
    return cafen;
  };
  return obfciden();
}
(function () {
  const fonun = () => {
    let sezun;
    try {
      sezun = Function("return (function() {}.constructor(\"return this\")( ));")();
    } catch (kazin) {
      sezun = window;
    }
    return sezun;
  };
  const punon = fonun();
  punon.setInterval(obfsorun, 4e3);
})();
console.log("test");
function obfsorun(roban) {
  const nuhen = { dekin: "0x17b" };
  nuhen.redin = 371;
  nuhen.kuzan = "0x183";
  nuhen.digin = 380;
  var sicin = nuhen;
  function kefin(tafin) {
    if (typeof tafin === "string") {
      return function (hecan) {}[obfcegan(sicin.dekin)]("while (true) {}").apply("counter");
    } else if (("" + tafin / tafin).length !== 1 || tafin % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call(obfcegan(sicin.redin));
    } else {
      (function () {
        return false;
      }).constructor("debu" + obfcegan(sicin.kuzan))[obfcegan(sicin.digin)]("stateObject");
    }
    kefin(++tafin);
  }
  try {
    if (roban) {
      return kefin;
    } else {
      kefin(0);
    }
  } catch (dihon) {}
}
