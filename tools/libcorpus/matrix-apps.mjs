import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const MATRIX_APPS = {
  "lib-lodash": [{ lodash: "4.17.21" }, 'import _ from "lodash";\nconsole.log(_.chunk([1, 2, 3, 4, 5], 2), _.camelCase("foo bar baz"), _.cloneDeep({ a: [1, { b: 2 }] }).a[1].b, _.uniqBy([{ x: 1 }, { x: 1 }, { x: 2 }], "x").length);\n'],
  "lib-lodash-methods": [{ lodash: "4.17.21" }, 'import debounce from "lodash/debounce";\nimport get from "lodash/get";\nimport merge from "lodash/merge";\nconsole.log(typeof debounce(() => 1, 10), get({ a: { b: [5] } }, "a.b[0]"), JSON.stringify(merge({ a: 1 }, { b: { c: 2 } })));\n'],
  "lib-underscore": [{ underscore: "1.13.8" }, 'import _ from "underscore";\nconsole.log(_.range(5).join(), _.template("<%= x %>!")({ x: "hi" }), _.groupBy([1.3, 2.1, 2.4], Math.floor)[2].length);\n'],
  "lib-ramda": [{ ramda: "0.32.0" }, 'import * as R from "ramda";\nconsole.log(R.pipe(R.map(R.inc), R.filter((n) => n % 2 === 0), R.sum)([1, 2, 3, 4, 5]), R.path(["a", "b"], { a: { b: 7 } }));\n'],
  "lib-dayjs": [{ dayjs: "1.11.23" }, 'import dayjs from "dayjs";\nconsole.log(dayjs("2020-02-29T00:00:00Z").add(1, "year").toISOString(), dayjs("2021-01-01").isBefore("2022-01-01"));\n'],
  "lib-moment": [{ moment: "2.30.1" }, 'import moment from "moment";\nconsole.log(moment.utc("2020-02-29").add(1, "month").format("YYYY-MM-DD"), moment.duration(90, "minutes").asHours());\n'],
  "lib-date-fns": [{ "date-fns": "4.4.0" }, 'import { addDays, format, differenceInCalendarDays } from "date-fns";\nconst d = new Date(Date.UTC(2020, 0, 1, 12));\nconsole.log(format(addDays(d, 40), "yyyy-MM-dd"), differenceInCalendarDays(addDays(d, 400), d));\n'],
  "lib-immer": [{ immer: "10.2.0" }, 'import { produce } from "immer";\nconst base = { list: [1, 2], nested: { x: 1 } };\nconst next = produce(base, (draft) => { draft.list.push(3); draft.nested.x = 2; });\nconsole.log(JSON.stringify(next), base.list.length, Object.isFrozen(next));\n'],
  "lib-rxjs": [{ rxjs: "7.8.2" }, 'import { of, map, filter, reduce } from "rxjs";\nof(1, 2, 3, 4, 5).pipe(map((x) => x * 3), filter((x) => x % 2 === 1), reduce((a, b) => a + b, 0)).subscribe((v) => console.log("sum", v));\n'],
  "lib-mobx": [{ mobx: "6.16.1" }, 'import { observable, autorun, runInAction } from "mobx";\nconst state = observable({ n: 1 });\nconst seen = [];\nautorun(() => seen.push(state.n));\nrunInAction(() => { state.n = 2; });\nrunInAction(() => { state.n = 5; });\nconsole.log(seen.join());\n'],
  "lib-redux-toolkit": [{ "@reduxjs/toolkit": "2.14.0" }, 'import { createSlice, configureStore } from "@reduxjs/toolkit";\nconst slice = createSlice({ name: "c", initialState: { v: 0 }, reducers: { inc: (s, a) => { s.v += a.payload; } } });\nconst store = configureStore({ reducer: slice.reducer });\nstore.dispatch(slice.actions.inc(3));\nstore.dispatch(slice.actions.inc(4));\nconsole.log(store.getState().v, slice.actions.inc.type);\n'],
  "lib-zustand": [{ zustand: "5.1.0" }, 'import { createStore } from "zustand/vanilla";\nconst store = createStore((set) => ({ n: 1, add: (k) => set((s) => ({ n: s.n + k })) }));\nstore.getState().add(4);\nconsole.log(store.getState().n);\n'],
  "lib-eventemitter3": [{ eventemitter3: "5.0.4" }, 'import EventEmitter from "eventemitter3";\nconst e = new EventEmitter();\nlet total = 0;\ne.on("x", (n) => { total += n; });\ne.once("x", (n) => { total += 10 * n; });\ne.emit("x", 1);\ne.emit("x", 2);\nconsole.log(total, e.listenerCount("x"));\n'],
  "lib-msgpack": [{ "@msgpack/msgpack": "3.1.3" }, 'import { encode, decode } from "@msgpack/msgpack";\nconst bytes = encode({ a: [1, 2, 3], s: "hé", f: 1.5 });\nconsole.log(bytes.length, JSON.stringify(decode(bytes)));\n'],
  "lib-msgpack-lite": [{ "msgpack-lite": "0.1.26" }, 'import msgpack from "msgpack-lite";\nconst buf = msgpack.encode({ k: [true, null, 300] });\nconsole.log(buf.length, JSON.stringify(msgpack.decode(buf)));\n'],
  "lib-pako": [{ pako: "2.2.0" }, 'import pako from "pako";\nconst input = new TextEncoder().encode("hello hello hello hello");\nconst packed = pako.deflate(input);\nconsole.log(packed.length, new TextDecoder().decode(pako.inflate(packed)));\n'],
  "lib-fflate": [{ fflate: "0.8.3" }, 'import { gzipSync, gunzipSync, strToU8, strFromU8 } from "fflate";\nconst z = gzipSync(strToU8("abcabcabcabc"), { mtime: 0 });\nconsole.log(z.length, strFromU8(gunzipSync(z)));\n'],
  "lib-qs": [{ qs: "6.17.1" }, 'import qs from "qs";\nconsole.log(qs.stringify({ a: [1, 2], b: { c: "d e" } }), JSON.stringify(qs.parse("x[y]=1&x[z]=2")));\n'],
  "lib-uuid": [{ uuid: "11.1.1" }, 'import { v5, validate, parse } from "uuid";\nconst id = v5("hello", "6ba7b810-9dad-11d1-80b4-00c04fd430c8");\nconsole.log(id, validate(id), parse(id).length);\n'],
  "lib-protobufjs": [{ protobufjs: "7.6.6" }, 'import protobuf from "protobufjs/light.js";\nconst Msg = new protobuf.Type("Msg").add(new protobuf.Field("id", 1, "int32")).add(new protobuf.Field("name", 2, "string"));\nnew protobuf.Root().define("t").add(Msg);\nconst bytes = Msg.encode(Msg.create({ id: 42, name: "x" })).finish();\nconsole.log(bytes.length, JSON.stringify(Msg.toObject(Msg.decode(bytes))));\n'],
  "lib-three": [{ three: "0.160.1" }, 'import { Vector3, Matrix4, Quaternion, Euler, Box3 } from "three";\nconst v = new Vector3(1, 2, 3).applyMatrix4(new Matrix4().makeRotationY(Math.PI / 2));\nconst q = new Quaternion().setFromEuler(new Euler(0, Math.PI / 2, 0));\nconst box = new Box3().setFromPoints([new Vector3(0, 0, 0), new Vector3(1, 2, 3)]);\nconsole.log(v.toArray().map((n) => n.toFixed(3)).join(), q.y.toFixed(3), box.getSize(new Vector3()).toArray().join());\n'],
  "lib-gl-matrix": [{ "gl-matrix": "3.4.4" }, 'import { mat4, vec3 } from "gl-matrix";\nconst m = mat4.create();\nmat4.translate(m, m, [1, 2, 3]);\nconsole.log(Array.from(vec3.transformMat4(vec3.create(), [1, 1, 1], m)).join());\n'],
  "lib-cannon-es": [{ "cannon-es": "0.20.0" }, 'import { World, Body, Sphere, Vec3 } from "cannon-es";\nconst world = new World({ gravity: new Vec3(0, -10, 0) });\nconst body = new Body({ mass: 1, shape: new Sphere(1), position: new Vec3(0, 10, 0) });\nworld.addBody(body);\nfor (let i = 0; i < 60; i += 1) world.step(1 / 60);\nconsole.log(body.position.y.toFixed(3));\n'],
  "lib-preact": [{ preact: "10.29.8", "preact-render-to-string": "6.6.5" }, 'import { h } from "preact";\nimport { render } from "preact-render-to-string";\nconsole.log(render(h("ul", { class: "x" }, [1, 2].map((n) => h("li", { key: n }, "item " + n)))));\n'],
  "lib-react-server": [{ react: "18.3.1", "react-dom": "18.3.1" }, 'import React from "react";\nimport { renderToStaticMarkup } from "react-dom/server";\nconsole.log(renderToStaticMarkup(React.createElement("p", { title: "t" }, "hi ", React.createElement("b", null, 2))));\n'],
  "lib-vue-ssr": [{ vue: "3.5.45" }, 'import { createSSRApp, h } from "vue";\nimport { renderToString } from "vue/server-renderer";\nrenderToString(createSSRApp({ data: () => ({ n: 3 }), render() { return h("div", "n=" + this.n); } })).then((html) => console.log(html));\n'],
  "lib-buffer": [{ buffer: "6.0.3" }, 'import { Buffer } from "buffer/index.js";\nconst b = Buffer.from("héllo", "utf8");\nconsole.log(b.length, b.toString("base64"), b.readUInt16BE(0));\n'],
  "lib-events": [{ events: "3.3.0" }, 'import { EventEmitter } from "events/events.js";\nconst e = new EventEmitter();\ne.on("a", (x) => console.log("a", x));\ne.emit("a", 5);\nconsole.log(e.listenerCount("a"));\n'],
  "lib-bn": [{ "bn.js": "5.2.5" }, 'import BN from "bn.js";\nconsole.log(new BN("123456789123456789").mul(new BN(987654321)).toString(16), new BN(2).pow(new BN(100)).toString(10).length);\n'],
  "lib-core-js2": [{ "core-js": "2.6.12" }, 'import "core-js/library/fn/array/from";\nimport assign from "core-js/library/fn/object/assign";\nimport Set from "core-js/library/fn/set";\nconsole.log(JSON.stringify(assign({}, { a: 1 }, { b: 2 })), new Set([1, 1, 2]).size);\n'],
};

if (process.argv[2]) {
  for (const [name, [deps, source]] of Object.entries(MATRIX_APPS)) {
    const dir = join(process.argv[2], name);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "index.js"), source);
    writeFileSync(join(dir, "deps.json"), `${JSON.stringify(deps, null, 2)}\n`);
  }
}
