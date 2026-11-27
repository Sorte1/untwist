import { createSignal } from "solid-js";
import { produce, batch, unwrap } from "./YProdLib.jsx";
export default function YProd() {
  const [s, setS] = createSignal({ n: 1 });
  return <div><p>{s().n}|{batch([1, 2, 3], 2).length}|{unwrap({ value: "v" })}</p><button onClick={() => setS(produce(s(), (d) => { d.n++; }))}>inc</button></div>;
}
