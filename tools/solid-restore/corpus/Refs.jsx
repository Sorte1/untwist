import { onMount } from "solid-js";

export default function Refs() {
  let input;
  let box;
  onMount(() => {
    input.value = "mounted";
    console.log(box.tagName);
  });
  return (
    <div>
      <input ref={input} />
      <div ref={(el) => (box = el)} class="box" />
      <span ref={(el) => el.setAttribute("data-ref", "yes")}>span</span>
    </div>
  );
}
