import { createSignal } from "solid-js";
export default function PCallInsert() {
  const [n, setN] = createSignal(1);
  const f = (k) => n() * k;
  const g = () => n() + 100;
  return (
    <div>
      <p>{/*@once*/ f(2)}</p>
      <p>{f(3)}</p>
      <p>{g()}</p>
      <p>{/*@once*/ g()}</p>
      <p>a{/*@once*/ g()}b{g()}c</p>
      <button onClick={() => setN(n() + 1)}>inc</button>
    </div>
  );
}
