import { mergeProps } from "solid-js";
import Badge from "./Badge.jsx";

export default function SpreadChild(props) {
  const merged = mergeProps({ tone: "soft" }, props);
  return (
    <section>
      <Badge {...merged} label="first" />
      <Badge label="second" {...props} />
    </section>
  );
}
