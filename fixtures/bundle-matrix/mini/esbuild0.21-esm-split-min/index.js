import { createRequire } from "module"; const require = createRequire(import.meta.url);
import{a as c,b as t}from"./chunk-VSBL6CES.chunk.js";var l=c(e=>{console.log("eval cjs");e.twice=function(o){return o*2}});var n=t(l());console.log("eval index "+(0,n.twice)(21));import("./lazy-4ZB7RQLQ.chunk.js").then(o=>console.log("lazy "+o.value));
