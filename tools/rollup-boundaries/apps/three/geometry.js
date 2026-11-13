import { BoxGeometry } from "three/src/geometries/BoxGeometry.js";
import { SphereGeometry } from "three/src/geometries/SphereGeometry.js";
import { MeshBasicMaterial } from "three/src/materials/MeshBasicMaterial.js";
import { Mesh } from "three/src/objects/Mesh.js";

export function box(size, color) {
  const mesh = new Mesh(new BoxGeometry(size, size, size), new MeshBasicMaterial({ color }));
  mesh.name = "box" + size;
  return mesh;
}

export function ball(radius) {
  const mesh = new Mesh(new SphereGeometry(radius, 12, 8), new MeshBasicMaterial({ color: 0x3366ff }));
  mesh.name = "ball" + radius;
  return mesh;
}
