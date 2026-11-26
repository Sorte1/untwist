import { createSignal, Index } from "solid-js";

export default function IndexList() {
  const [names, setNames] = createSignal(["x", "y"]);
  return (
    <ol>
      <Index each={names()}>{(name, i) => <li onClick={() => setNames(names().map((n, j) => (j === i ? n + "!" : n)))}>{name()}</li>}</Index>
    </ol>
  );
}
