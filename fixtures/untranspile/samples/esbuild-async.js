var u = "x";
function fetch(v) { return Promise.resolve({ json() { return "json:" + v; } }); }
var l=(c,a,t)=>new Promise((i,r)=>{var s=e=>{try{n(t.next(e))}catch(l){r(l)}},v=e=>{try{n(t.throw(e))}catch(l){r(l)}},n=e=>e.done?i(e.value):Promise.resolve(e.value).then(s,v);n((t=t.apply(c,a)).next())});
function go(){return l(this,null,function*(){return(yield fetch(u)).json()})}
go().then((v) => console.log(v));
