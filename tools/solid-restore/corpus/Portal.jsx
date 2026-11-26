import { Portal } from "solid-js/web";

export default function PortalTest() {
  return (
    <div>
      <p>inside</p>
      <Portal mount={document.querySelector("main")}>
        <p class="portal">teleported</p>
      </Portal>
    </div>
  );
}
