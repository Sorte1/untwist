import { createSignal } from "solid-js";
import { Dynamic } from "solid-js/web";

function Red(props) {
  return <b class="red">{props.text}</b>;
}

export default function DynamicTag() {
  const [tag, setTag] = createSignal("h2");
  return (
    <div>
      <Dynamic component={tag()} class="dyn">
        heading
      </Dynamic>
      <Dynamic component={Red} text="red" />
      <button onClick={() => setTag(tag() === "h2" ? "h3" : "h2")}>swap</button>
    </div>
  );
}
