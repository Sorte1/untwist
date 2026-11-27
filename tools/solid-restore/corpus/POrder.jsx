import { createSignal } from "solid-js";
function Kid(props) { console.log("kid", props.name); return <i ref={props.ref}>{props.children}</i>; }
export default function POrder(props) {
  const [n, setN] = createSignal(1);
  const log = (k) => (console.log(k), k);
  let r1, r2;
  return (
    <div ref={(e) => console.log("ref div", e.childNodes.length)} title={/*@once*/ log("t1")} {...{ id: log("spread") }} lang={/*@once*/ log("t2")}>
      {log("c1")}
      <Kid name="k1" ref={r1}>{log("kc1")}</Kid>
      <span ref={r2} onClick={() => console.log(r1.tagName, r2.tagName)}>{log("c2")}</span>
      <Kid name="k2">{n()}</Kid>
      <button onClick={() => (r2.click(), setN(n() + 1))}>inc</button>
    </div>
  );
}
