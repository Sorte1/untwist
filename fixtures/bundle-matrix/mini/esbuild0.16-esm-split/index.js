import { createRequire } from "module"; const require = createRequire(import.meta.url);
import {
  __commonJS,
  __toESM
} from "./chunk-UMZ43YGM.chunk.js";

// .src-mini-esm/cjs.js
var require_cjs = __commonJS({
  ".src-mini-esm/cjs.js"(exports) {
    console.log("eval cjs");
    exports.twice = function(n) {
      return n * 2;
    };
  }
});

// .src-mini-esm/index.js
var import_cjs = __toESM(require_cjs());
console.log("eval index " + (0, import_cjs.twice)(21));
import("./lazy-BC6KBO4V.chunk.js").then((lazy) => console.log("lazy " + lazy.value));
