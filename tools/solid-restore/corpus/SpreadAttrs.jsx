import { createSignal, splitProps } from "solid-js";

export default function SpreadAttrs(props) {
  const [local, rest] = splitProps(props, ["label"]);
  const [extra, setExtra] = createSignal({ "data-x": "1" });
  return (
    <div>
      <button {...rest} {...extra()} class="btn" onClick={() => setExtra({ "data-x": "2", title: "two" })}>
        {local.label}
      </button>
    </div>
  );
}
