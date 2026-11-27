import { createSignal } from "solid-js";
const [g, setG] = createSignal(1);
function Kid() { return <b>{g()}</b>; }
export default function ZModDestr() { return <p><Kid /><button onClick={() => setG((v) => v + 1)}>inc</button></p>; }
