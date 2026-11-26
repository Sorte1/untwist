import { createSignal } from "solid-js";

function Pair(props) {
  return (
    <>
      <dt>{props.term}</dt>
      <dd>{props.children}</dd>
    </>
  );
}

export default function Fragment() {
  const [n, setN] = createSignal(1);
  return (
    <>
      <dl>
        <Pair term="one">first</Pair>
        <Pair term="two">second {n()}</Pair>
      </dl>
      text after
      <button onClick={() => setN(n() * 2)}>double</button>
    </>
  );
}
