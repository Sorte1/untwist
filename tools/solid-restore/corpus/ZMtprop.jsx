import { createSignal } from "solid-js";
function Kid(props) { return <i>{props.children}{props.x}</i>; }
export default function ZMtprop() {
  const [n, setN] = createSignal(0);
  const a = () => { console.log("a"); return n() + "a"; };
  return <div><Kid x={n() ? a() : "zero"} /><button onClick={() => setN(n() + 1)}>inc</button></div>;
}
