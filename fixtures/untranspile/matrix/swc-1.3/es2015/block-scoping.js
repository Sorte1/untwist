"use strict";
const fns = [];
for(let i = 0; i < 3; i++){
    fns.push(()=>i);
}
console.log(fns.map((f)=>f()).join(","));
const later = [];
for(let i = 0; i < 4; i++){
    if (i === 1) continue;
    if (i === 3) break;
    later.push(function() {
        return i * 10;
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
    let x = "inner";
    const y = x + "!";
    console.log(x, y);
}console.log(x);
const ws = [];
let j = 0;
while(j < 3){
    let k = j;
    ws.push(()=>k);
    j++;
}
console.log(ws.map((f)=>f()).join(","));
for(let i = 0, n = 2; i < n; i++){
    setTimeout(()=>console.log("timer", i), 0);
}
switch(1){
    case 1:
        {
            let s = "case";
            console.log(s);
        }
}
