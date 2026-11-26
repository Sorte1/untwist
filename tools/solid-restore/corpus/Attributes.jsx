import { createSignal } from "solid-js";

export default function Attributes(props) {
  const [on, setOn] = createSignal(false);
  return (
    <div>
      <a href={props.href} target="_blank" rel="noopener" aria-label={on() ? "on" : "off"}>
        link
      </a>
      <input type="checkbox" checked={on()} disabled={props.locked} />
      <span class={on() ? "active" : "idle"} data-state={on() ? 1 : 0} tabindex="0">state</span>
      <div class={`box ${props.kind}`} id={props.id} hidden={!on()} />
      <button type="button" onClick={() => setOn(!on())}>toggle</button>
    </div>
  );
}
