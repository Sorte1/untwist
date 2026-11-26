import { createSignal, ErrorBoundary, Show } from "solid-js";

function Bomb(props) {
  if (props.armed) throw new Error("boom");
  return <p>safe</p>;
}

export default function Boundary() {
  const [armed, setArmed] = createSignal(false);
  return (
    <div>
      <ErrorBoundary fallback={(err, reset) => <p class="err" onClick={reset}>{err.message}</p>}>
        <Show when={armed()} fallback={<Bomb armed={false} />}>
          <Bomb armed />
        </Show>
      </ErrorBoundary>
      <button onClick={() => setArmed(true)}>arm</button>
    </div>
  );
}
