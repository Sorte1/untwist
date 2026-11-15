export const h = function (c) { console.log(typeof c); return c; };
const r = h(3);
console.log(typeof r);
export let k = (d) => { console.log(typeof d); return d; };
k(4);
