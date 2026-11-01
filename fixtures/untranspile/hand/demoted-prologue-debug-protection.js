function dpf(ret) {
  function debuggerProtection(counter) {
    if (typeof counter === 'string') {
      return function (arg) {}['constructor']('while (true) {}')['apply']('counter');
    } else {
      debugger;
    }
    debuggerProtection(++counter);
  }
  try {
    if (ret) {
      return debuggerProtection;
    } else {
      debuggerProtection(0);
    }
  } catch (y) {}
}
'use strict';
sloppyGlobal = 1;
console.log(typeof sloppyGlobal, (function () { return this === undefined; })());
