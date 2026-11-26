import { createSignal, Switch, Match } from "solid-js";

export default function SwitchMatch() {
  const [n, setN] = createSignal(0);
  return (
    <div>
      <Switch fallback={<p>many</p>}>
        <Match when={n() === 0}>
          <p>zero</p>
        </Match>
        <Match when={n() === 1}>
          <p>one</p>
        </Match>
      </Switch>
      <button onClick={() => setN(n() + 1)}>next</button>
    </div>
  );
}
