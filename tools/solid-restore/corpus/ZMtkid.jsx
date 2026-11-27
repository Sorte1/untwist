import { createSignal } from "solid-js";
function Kid(props) { return <i>{props.children}{props.x}</i>; }
export default function ZMtkid() {
  const [n, setN] = createSignal(0);
  const a = () => { console.log("a"); return n() + "a"; };
  return <div><Kid>{n() ? a() : "zero"}</Kid><button onClick={() => setN(n() + 1)}>inc</button></div>;
}
