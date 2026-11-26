import { createSignal } from "solid-js";

export default function Svg() {
  const [r, setR] = createSignal(5);
  return (
    <div>
      <svg width="40" height="40" viewBox="0 0 40 40">
        <circle cx="20" cy="20" r={r()} fill="red" />
        <path d="M0 0L10 10" stroke-width="2" />
        <text x="2" y="38">{r()}</text>
      </svg>
      <button onClick={() => setR(r() + 5)}>grow</button>
    </div>
  );
}
