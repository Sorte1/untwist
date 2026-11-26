import { createSignal } from "solid-js";

export default function RawHtml(props) {
  const [html, setHtml] = createSignal("<b>bold</b>");
  return (
    <div>
      <div class="raw" innerHTML={html()} />
      <div class="txt" textContent={props.text} />
      <button onClick={() => setHtml("<i>it</i>")}>change</button>
    </div>
  );
}
