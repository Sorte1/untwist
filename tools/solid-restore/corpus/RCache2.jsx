import { createSignal } from "solid-js";
const makeStore = () => { const [v, set] = createSignal(0); return { v, bump: () => set(v() + 1) }; };
const store = makeStore();
const ids = new Map();
const idOf = (k) => { if (!ids.has(k)) ids.set(k, ids.size + 1); return ids.get(k); };
function Alpha() { return <b onClick={store.bump}>{store.v()}:{idOf("alpha")}</b>; }
function Beta() { return <i>{store.v()}:{idOf("beta")}:{idOf("alpha")}</i>; }
export default function RCache2() {
  return <div><Alpha /><Beta /><button onClick={() => store.bump()}>inc</button></div>;
}
