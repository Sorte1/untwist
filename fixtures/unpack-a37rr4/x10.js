"use strict";
(() => { "use strict";  var __webpack_modules__ = ({ 1: (module, exports, __webpack_require__) => { console.log("m1", __webpack_require__(2)); }, 2: (module) => { console.log("m2"); module.exports = "two"; } }); console.log("between");
var __webpack_module_cache__ = {}; 
function __webpack_require__(moduleId) {  var cachedModule = __webpack_module_cache__[moduleId]; if (cachedModule !== undefined) { return cachedModule.exports; } var module = __webpack_module_cache__[moduleId] = { exports: {} }; __webpack_modules__[moduleId](module, module.exports, __webpack_require__); return module.exports; }
 var __webpack_exports__ = __webpack_require__(1);  })();

