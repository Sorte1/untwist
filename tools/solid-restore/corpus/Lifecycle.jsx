import { createSignal, onMount, onCleanup, Show } from "solid-js";

function Ticker(props) {
  onMount(() => console.log("mount", props.name));
  onCleanup(() => console.log("cleanup", props.name));
  return <p>{props.name}</p>;
}

export default function Lifecycle() {
  const [shown, setShown] = createSignal(true);
  return (
    <div>
      <Show when={shown()}>
        <Ticker name="tick" />
      </Show>
      <button onClick={() => setShown(!shown())}>toggle</button>
    </div>
  );
}
