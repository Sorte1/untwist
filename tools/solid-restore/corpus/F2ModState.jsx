import { createSignal } from "solid-js";
const [g, setG] = createSignal(1);
setG(5);
const list = [];
for (let i = 0; i < 3; i++) list.push(i);
function Kid() { return <b>{g()}</b>; }
Kid.label = "lbl";
export default function F2ModState() {
  return <p><Kid />|{Kid.label}|{list.join()}</p>;
}
