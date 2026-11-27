const mk = (k) => { console.log(k); return () => console.log("h" + k); };
const s = (k) => { console.log(k); return k; };
export default function ZEvOrder() {
  return <div><button onClick={mk("c")} title={/*@once*/ s("t")} on:foo={mk("f")} id={/*@once*/ s("i")}>b</button></div>;
}
