import { createSignal } from "solid-js";
const [g, setG] = createSignal(1);
const first = setG(5);
function Kid() { return <b>{g()}</b>; }
export default function ZModDecl() { return <p><Kid /></p>; }
