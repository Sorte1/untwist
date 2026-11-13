import { Raycaster } from "three/src/core/Raycaster.js";
import { Vector2 } from "three/src/math/Vector2.js";

export function pick(world, x, y) {
  const raycaster = new Raycaster();
  raycaster.setFromCamera(new Vector2(x, y), world.camera);
  return raycaster.intersectObjects(world.scene.children, true).map((hit) => hit.object.name + "@" + hit.distance.toFixed(3));
}
