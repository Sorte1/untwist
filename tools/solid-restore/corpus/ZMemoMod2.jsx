import { createSignal } from "solid-js";
import { wrap } from "./ZWrapMod.jsx";
export default function ZMemoMod2() {
  const [n, setN] = createSignal(1);
  const w = wrap(() => n() * 2);
  return <div><p>{w()}</p><button onClick={() => setN(n() + 1)}>inc</button></div>;
}
