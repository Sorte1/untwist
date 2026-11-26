import { createSignal } from "solid-js";

export default function PropAttr() {
  const [v, setV] = createSignal("x");
  return (
    <div>
      <input prop:value={v()} attr:data-v={v()} />
      <my-element attr:size={v()} prop:items={[1, 2]} class="custom" />
      <button onClick={() => setV(v() + "y")}>more</button>
    </div>
  );
}
