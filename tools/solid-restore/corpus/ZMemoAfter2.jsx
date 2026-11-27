import { createSignal } from "solid-js";
export default function ZMemoAfter2() {
  const [n, setN] = createSignal(1);
  const w = wrap(() => n() * 2);
  return <div><p>{w()}</p><button onClick={() => setN(n() + 1)}>inc</button></div>;
}
const log = (f) => { console.log("wrapped"); return f; };
const wrap = (fn) => log(() => fn());
