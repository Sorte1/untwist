import { createSignal } from "solid-js";
export default function F3AttrOrder() {
  const [n, setN] = createSignal(1);
  const log = (k) => (console.log(k), k + n());
  return <button style={{ color: log("red") }} title={log("t")} onClick={() => setN(n() + 1)}>inc</button>;
}
