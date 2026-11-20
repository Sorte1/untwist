function T(f){ try { return String(f()) } catch (e) { return e.name } }
globalThis.act9 = async function(){
const { Worker: W9 } = await import("node:worker_threads"); console.log(typeof W9);
};
export {};
