import { createSignal } from "solid-js";
import Badge from "./Badge.jsx";

export default function Parent() {
  const [n, setN] = createSignal(0);
  return (
    <div class="parent">
      <Badge label="static" />
      <Badge label={`count ${n()}`} tone={n() > 1 ? "hot" : undefined}>
        <em>child {n()}</em>
      </Badge>
      <Badge label="text">plain text</Badge>
      <button onClick={() => setN(n() + 1)}>more</button>
    </div>
  );
}
