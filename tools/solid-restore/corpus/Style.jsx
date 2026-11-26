import { createSignal } from "solid-js";

export default function Style(props) {
  const [size, setSize] = createSignal(10);
  return (
    <div>
      <p style={{ color: props.color, "font-size": `${size()}px`, "--gap": "4px" }}>styled</p>
      <p style="margin: 0; padding: 2px">plain</p>
      <p style={`width: ${size() * 2}px`}>string</p>
      <button onClick={() => setSize(size() + 5)}>grow</button>
    </div>
  );
}
