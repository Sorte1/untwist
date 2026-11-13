"use strict";
const fns = [];
for(let i = 0; i < 3; i++){
    fns.push(()=>i);
}
console.log(fns.map((f)=>f()).join(","));
const later = [];
for(let i1 = 0; i1 < 4; i1++){
    if (i1 === 1) continue;
    if (i1 === 3) break;
    later.push(function() {
        return i1 * 10;
    });
}
console.log(later.map((f)=>f()).join(","));
function find(list) {
    for (const item of list){
        const handler = ()=>item.id;
        if (item.ok) return handler();
    }
    return -1;
}
console.log(find([
    {
        id: 1
    },
    {
        id: 2,
        ok: true
    }
]));
let x = "outer";
{
    let x1 = "inner";
    const y = x1 + "!";
    console.log(x1, y);
}console.log(x);
const ws = [];
let j = 0;
while(j < 3){
    let k = j;
    ws.push(()=>k);
    j++;
}
console.log(ws.map((f)=>f()).join(","));
for(let i2 = 0, n = 2; i2 < n; i2++){
    setTimeout(()=>console.log("timer", i2), 0);
}
switch(1){
    case 1:
        {
            let s = "case";
            console.log(s);
        }
}
