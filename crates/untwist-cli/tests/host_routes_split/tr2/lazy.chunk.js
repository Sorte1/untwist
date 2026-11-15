let d; try { d = (await import("./a.cjs.js")).default; } catch (e) { d = e.code; } export default "chunk " + d;
