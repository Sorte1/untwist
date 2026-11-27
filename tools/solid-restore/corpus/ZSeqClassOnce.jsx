import { createSignal } from "solid-js";
function Kid(props) { return <i>{props.a}{props.children}</i>; }
const tag = (s) => s[0] + "!";
export default function ZSeqClassOnce() {
  const [n, setN] = createSignal(1);
  let v = 3, w = 0;
  const key = "a";
  const rest = { lang: "en" };
  const f = (k) => { console.log("f", k); return n() * k; };
  const o = { a: 1, get g() { console.log("g"); return n() * 10; } };
  return <div><p classList={{ on: /*@once*/ (v, f(3)) }}>x</p><button class="inc" onClick={() => { v++; setN(n() + 1); }}>inc</button></div>;
}
