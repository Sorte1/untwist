const safe = (f) => { console.log("safe"); return f(); };
const schedule = (f) => { console.log("sched"); f(); };
export function later(fn) { schedule(() => safe(fn)); }
export default function YLaterLib() { return null; }
