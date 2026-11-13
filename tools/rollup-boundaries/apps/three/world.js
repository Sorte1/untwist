import { Scene } from "three/src/scenes/Scene.js";
import { Group } from "three/src/objects/Group.js";
import { PerspectiveCamera } from "three/src/cameras/PerspectiveCamera.js";
import { box, ball } from "./geometry.js";

export function buildWorld() {
  const scene = new Scene();
  const group = new Group();
  group.add(box(1, 0xff0000));
  group.add(ball(0.5));
  group.children[1].position.set(3, 0, 0);
  group.rotation.y = Math.PI / 4;
  scene.add(group);
  const camera = new PerspectiveCamera(60, 16 / 9, 0.1, 100);
  camera.position.set(0, 2, 8);
  camera.lookAt(0, 0, 0);
  camera.updateMatrixWorld(true);
  scene.updateMatrixWorld(true);
  return { scene, group, camera };
}
