import { children, createSignal } from "solid-js";

function Wrap(props) {
  const resolved = children(() => props.children);
  return <div class="wrap" data-count={resolved.toArray().length}>{resolved()}</div>;
}

export default function ChildrenHelper() {
  const [n, setN] = createSignal(1);
  return (
    <section>
      <Wrap>
        <i>one</i>
        <i>two {n()}</i>
      </Wrap>
      <button onClick={() => setN(n() + 1)}>inc</button>
    </section>
  );
}
