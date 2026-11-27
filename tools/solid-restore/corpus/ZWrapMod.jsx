export const log = (f) => { console.log("wrapped"); return f; };
export const wrap = (fn) => log(() => fn());
export default function ZWrapMod() { return null; }
