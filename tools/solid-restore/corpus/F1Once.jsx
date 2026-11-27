import { createSignal } from "solid-js";
function Kid(props) { return <i>{props.a}</i>; }
export default function F1Once() {
  const [n, setN] = createSignal(1);
  const f = (k) => n() * k;
  return (
    <div>
      <p title={/*@once*/ f(1)}>{/*@once*/ f(2)}</p>
      <Kid a={/*@once*/ f(3)} />
      <button onClick={() => setN(n() + 1)}>inc</button>
    </div>
  );
}
