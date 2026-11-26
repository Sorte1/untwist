import { createSignal } from "solid-js";

export default function ClassList() {
  const [n, setN] = createSignal(0);
  return (
    <div>
      <p class="base" classList={{ odd: n() % 2 === 1, even: n() % 2 === 0, big: n() > 2 }}>
        {n()}
      </p>
      <span classList={{ fixed: true }}>fixed</span>
      <button onClick={() => setN(n() + 1)}>next</button>
    </div>
  );
}
