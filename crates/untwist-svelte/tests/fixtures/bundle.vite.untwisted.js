var sn = Array.isArray;
var Wn = Array.prototype.indexOf;
var Ke = Array.prototype.includes;
var dt = Array.from;
var ln = Object.defineProperty;
var Ee = Object.getOwnPropertyDescriptor;
var Xn = Object.prototype;
var Zn = Array.prototype;
var Jn = Object.getPrototypeOf;
var Ut = Object.isExtensible;
function Be(e) {
  return typeof e == "function";
}
const st = () => {};
function Qn(e) {
  for (let t = 0; t < e.length; t++) {
    e[t]();
  }
}
function fn() {
  let e;
  let t;
  const n = new Promise((r, i) => {
    e = r;
    t = i;
  });
  return {
    promise: n,
    resolve: e,
    reject: t
  };
}
function Gt(e, t, n = false) {
  if (e === undefined) {
    if (n) {
      return t();
    } else {
      return t;
    }
  } else {
    return e;
  }
}
function er(e, t) {
  if (Array.isArray(e)) {
    return e;
  }
  if (!(Symbol.iterator in e)) {
    return Array.from(e);
  }
  const n = [];
  for (const r of e) {
    n.push(r);
    if (n.length === t) {
      break;
    }
  }
  return n;
}
const M = 2;
const Ne = 4;
const ht = 8;
const on = 16777216;
const G = 16;
const re = 32;
const ve = 64;
const Et = 128;
const Dt = 256;
const X = 512;
const T = 1024;
const k = 2048;
const B = 4096;
const F = 8192;
const O = 16384;
const Le = 32768;
const lt = 33554432;
const De = 65536;
const ft = 131072;
const tr = 262144;
const $e = 524288;
const nr = 1048576;
const ee = 33554432;
const ot = 2097152;
const Oe = 4194304;
const de = 8388608;
const nt = Symbol("$state");
const un = Symbol("component");
const rr = Symbol("legacy props");
const ir = Symbol("attributes");
const kt = Symbol("class");
const sr = Symbol("style");
const xt = Symbol("text");
const rt = Symbol("form reset");
const Ze = new class extends Error {
  name = "StaleReactionError";
  message = "The reaction that called `getAbortSignal()` was re-run or destroyed";
}();
const lr = 1;
const fr = 2;
const an = 4;
const or = 8;
const ur = 16;
const ar = 1;
const cr = 2;
const cn = {};
const S = Symbol("uninitialized");
function dr() {
  console.warn("https://svelte.dev/e/derived_inert");
}
function hr() {
  console.warn("https://svelte.dev/e/svelte_boundary_reset_noop");
}
function dn(e) {
  return e === this.v;
}
function vr(e, t) {
  if (e != e) {
    return t == t;
  } else {
    return e !== t || e !== null && typeof e == "object" || typeof e == "function";
  }
}
function hn(e) {
  return !vr(e, this.v);
}
function _r() {
  throw new Error("https://svelte.dev/e/async_derived_orphan");
}
function pr(e, t, n) {
  throw new Error("https://svelte.dev/e/each_key_duplicate");
}
function gr() {
  throw new Error("https://svelte.dev/e/effect_update_depth_exceeded");
}
function mr() {
  throw new Error("https://svelte.dev/e/state_descriptors_fixed");
}
function wr() {
  throw new Error("https://svelte.dev/e/state_prototype_fixed");
}
function br() {
  throw new Error("https://svelte.dev/e/state_unsafe_mutation");
}
function yr() {
  throw new Error("https://svelte.dev/e/svelte_boundary_reset_onerror");
}
let Z = null;
function Pe(e) {
  Z = e;
}
function vt(e, t = false, n) {
  Z = {
    p: Z,
    i: false,
    c: null,
    e: null,
    s: e,
    x: null,
    r: y,
    l: null
  };
}
function _t(e) {
  const t = Z;
  const n = t.e;
  if (n !== null) {
    t.e = null;
    for (const r of n) {
      Hr(r);
    }
  }
  t.i = true;
  Z = t.p;
  return vn(e);
}
function vn(e = {}) {
  ln(e, un, { value: true });
  return e;
}
function _n() {
  return true;
}
let be = [];
function pn() {
  const e = be;
  be = [];
  Qn(e);
}
function ae(e) {
  if (be.length === 0 && !Ye) {
    var t = be;
    queueMicrotask(() => {
      if (t === be) {
        pn();
      }
    });
  }
  be.push(e);
}
function Er() {
  while (be.length > 0) {
    pn();
  }
}
const kr = -7169;
function x(e, t) {
  e.f = e.f & kr | t;
}
function Pt(e) {
  if ((e.f & X) !== 0 || e.deps === null) {
    x(e, T);
  } else {
    x(e, B);
  }
}
function gn(e, t) {
  if ((e.f & k) !== 0) {
    t.set(e, k);
  } else if ((e.f & B) !== 0 && t.get(e) !== k) {
    t.set(e, B);
  }
  x(e, T);
}
let Kt = false;
function xr() {
  if (!Kt) {
    Kt = true;
    document.addEventListener("reset", (e) => {
      Promise.resolve().then(() => {
        if (!e.defaultPrevented) {
          for (const t of e.target.elements) {
            t[rt]?.();
          }
        }
      });
    }, { capture: true });
  }
}
function Je(e) {
  const t = w;
  const n = y;
  z(null);
  se(null);
  try {
    return e();
  } finally {
    z(t);
    se(n);
  }
}
function mn(e, t, n, r = n) {
  e.addEventListener(t, () => Je(n));
  const i = e[rt];
  if (i) {
    e[rt] = () => {
      i();
      r(true);
    };
  } else {
    e[rt] = () => r(true);
  }
  xr();
}
function Sr(e, t, n, r) {
  const i = Ft;
  const s = e.filter((h) => !h.settled);
  var l = t.map(i);
  if (n.length === 0 && s.length === 0) {
    r(l);
    return;
  }
  var f = y;
  var o = Tr();
  const u = s.length === 1 ? s[0].promise : s.length > 1 ? Promise.all(s.map((h) => h.promise)) : null;
  function a(h) {
    if ((f.f & O) === 0) {
      o();
      try {
        r([...l, ...h]);
      } catch (v) {
        Q(v, f);
      }
      ut();
    }
  }
  var c = wn();
  if (n.length === 0) {
    u.then(() => a([])).finally(c);
    return;
  }
  function d() {
    Promise.all(n.map((h) => Ar(h))).then(a).catch((h) => Q(h, f)).finally(c);
  }
  if (u) {
    u.then(() => {
      if ((f.f & O) !== 0) {
        c();
        return;
      }
      o();
      d();
      ut();
    });
  } else {
    d();
  }
}
function Tr() {
  var e = y;
  var t = w;
  var n = Z;
  var r = m;
  return function (s = true) {
    se(e);
    z(t);
    Pe(n);
    if (s && (e.f & O) === 0) {
      r?.activate();
      r?.apply();
    }
  };
}
function ut(e = true) {
  se(null);
  z(null);
  Pe(null);
  if (e) {
    m?.deactivate();
  }
}
function wn() {
  const e = y;
  const t = e.b;
  const n = m;
  const r = !!t?.is_rendered();
  t?.update_pending_count(1, n);
  n.increment(r, e);
  return () => {
    t?.update_pending_count(-1, n);
    n.decrement(r, e);
  };
}
function Ft(e) {
  const t = M | k;
  if (y !== null) {
    y.f |= $e;
  }
  return {
    ctx: Z,
    deps: null,
    effects: null,
    equals: dn,
    f: t,
    fn: e,
    reactions: null,
    rv: 0,
    v: S,
    wv: 0,
    parent: y,
    ac: null
  };
}
const qe = Symbol("obsolete");
function Ar(e, t, n) {
  const r = y;
  if (r === null) {
    _r();
  }
  let i = undefined;
  const s = Se(S);
  const l = !w;
  const f = new Set();
  zr(() => {
    const o = y;
    const u = fn();
    i = u.promise;
    try {
      Promise.resolve(e()).then(u.resolve, (h) => {
        if (h !== Ze) {
          u.reject(h);
        }
      }).finally(ut);
    } catch (h) {
      u.reject(h);
      ut();
    }
    const a = m;
    if (l) {
      if ((o.f & Le) !== 0) {
        var c = wn();
      }
      if (r.b?.is_rendered()) {
        a.async_deriveds.get(o)?.reject(qe);
      } else {
        for (const h of f.values()) {
          h.reject(qe);
        }
      }
      f.add(u);
      a.async_deriveds.set(o, u);
    }
    const d = (h, v = undefined) => {
      c?.();
      f.delete(u);
      if (v !== qe) {
        a.activate();
        if (v) {
          s.f |= de;
          Fe(s, v);
        } else {
          if ((s.f & de) !== 0) {
            s.f ^= de;
          }
          Fe(s, h);
        }
        a.deactivate();
      }
    };
    u.promise.then(d, (h) => d(null, h || "unknown"));
  });
  Br(() => {
    for (const o of f) {
      o.reject(qe);
    }
  });
  return new Promise((o) => {
    function u(a) {
      function c() {
        if (a === i) {
          o(s);
        } else {
          u(i);
        }
      }
      a.then(c, c);
    }
    u(i);
  });
}
function It(e) {
  const t = Ft(e);
  Ln(t);
  return t;
}
function St(e) {
  const t = Ft(e);
  t.equals = hn;
  return t;
}
function Cr(e) {
  const t = e.effects;
  if (t !== null) {
    e.effects = null;
    for (let n = 0; n < t.length; n += 1) {
      L(t[n]);
    }
  }
}
function Lt(e) {
  const n = y;
  const r = e.parent;
  if (!Te && r !== null && e.v !== S && (r.f & (O | F)) !== 0) {
    dr();
    return e.v;
  }
  se(r);
  let t;
  try {
    Cr(e);
    t = Hn(e);
  } finally {
    se(n);
  }
  return t;
}
function bn(e) {
  const t = Lt(e);
  if (!e.equals(t) && (e.wv = jn(), (!m?.is_fork || e.deps === null) && (m !== null ? (m.capture(e, t, true), Me?.capture(e, t, true)) : e.v = t, e.deps === null))) {
    x(e, T);
    return;
  }
  if (!Te) {
    if (K !== null) {
      if (Bt() || m?.is_fork) {
        K.set(e, t);
      }
    } else {
      Pt(e);
    }
  }
}
function Rr(e) {
  if (e.effects !== null) {
    for (const t of e.effects) {
      if (t.teardown || t.ac) {
        t.teardown?.();
        if (t.ac !== null) {
          Je(() => {
            t.ac.abort(Ze);
            t.ac = null;
          });
        }
        if (t.fn !== null) {
          t.teardown = st;
        }
        Xe(t, 0);
        Vt(t);
      }
    }
  }
}
function yn(e) {
  if (e.effects !== null) {
    for (const t of e.effects) {
      if (t.teardown && t.fn !== null) {
        Ie(t);
      }
    }
  }
}
let pt = null;
let Re = null;
let m = null;
let Me = null;
let K = null;
let Tt = null;
let Ye = false;
let gt = false;
let Ue = null;
let it = null;
var Wt = 0;
let Or = 1;
class _e {
  id = Or++;
  started = false;
  linked = true;
  prev = null;
  next = null;
  async_deriveds = new Map();
  current = new Map();
  previous = new Map();
  #t = new Set();
  #o = new Set();
  #e = 0;
  #l = new Map();
  #n = null;
  #r = [];
  #s = [];
  #i = new Map();
  #f = new Map();
  #h = new Set();
  is_fork = false;
  #u = false;
  constructor() {
    if (Re === null) {
      pt = Re = this;
    } else {
      Re.next = this;
      this.prev = Re;
    }
    Re = this;
  }
  #v() {
    if (this.is_fork) {
      return true;
    }
    for (const t of this.#l.keys()) {
      if (!this.#g(t)) {
        return true;
      }
    }
    return false;
  }
  #g(t) {
    for (let n = t; n !== null; n = n.parent) {
      if (this.#f.has(n)) {
        return true;
      }
    }
    return false;
  }
  skip_effect(t) {
    if (!this.#f.has(t)) {
      this.#f.set(t, {
        d: [],
        m: []
      });
    }
    this.#h.delete(t);
  }
  unskip_effect(t, n = (r) => this.schedule(r)) {
    const r = this.#f.get(t);
    if (r) {
      this.#f.delete(t);
      for (var i of r.d) {
        x(i, k);
        n(i);
      }
      for (i of r.m) {
        x(i, B);
        n(i);
      }
    }
    this.#h.add(t);
  }
  #a() {
    const t = [];
    for (const s of this.#r) {
      if ((s.f & O) === 0 && (s.f & (k | B)) !== 0) {
        for (var n = s, r = false; n.parent !== null;) {
          n = n.parent;
          const i = n.f;
          if ((i & (ve | re)) !== 0) {
            if ((i & T) === 0) {
              r = true;
              break;
            }
            n.f ^= T;
          }
        }
        if (!r) {
          t.push(n);
        }
      }
    }
    this.#r = [];
    return t;
  }
  #_() {
    this.started = true;
    for (const [f, o] of this.#i) {
      x(f, o);
      this.schedule(f);
    }
    this.apply();
    const t = Ue = [];
    const n = [];
    const r = it = [];
    while (this.#r.length > 0) {
      if (Wt++ > 1e3) {
        this.#d();
        Nr();
      }
      for (const f of this.#a()) {
        try {
          this.#w(f, t, n);
        } catch (o) {
          xn(f);
          if (!this.#v()) {
            this.discard();
          }
          throw o;
        }
      }
    }
    m = null;
    if (r.length > 0) {
      const i = _e.ensure();
      for (const f of r) {
        i.schedule(f);
      }
    }
    Ue = null;
    it = null;
    if (this.#v()) {
      this.#p(n);
      this.#p(t);
      for (const [f, o] of this.#f) {
        kn(f, o);
      }
      if (r.length > 0) {
        m.#_();
      }
      return;
    }
    const s = this.#E();
    if (s) {
      this.#p(n);
      this.#p(t);
      s.#c(this);
      return;
    }
    this.#i.clear();
    for (const f of this.#t) {
      f(this);
    }
    this.#t.clear();
    Me = this;
    Xt(n);
    Xt(t);
    Me = null;
    this.#n?.resolve();
    const l = m;
    if (this.#e === 0 && (this.#r.length === 0 || l !== null)) {
      this.#d();
    }
    if (l !== null) {
      te.clear();
      l.#_();
    }
  }
  #w(t, n, r) {
    t.f ^= T;
    for (let i = t.first; i !== null;) {
      const s = i.f;
      const l = (s & (re | ve)) !== 0;
      const f = l && (s & T) !== 0;
      const o = f || (s & F) !== 0 || this.#f.has(i);
      if (!o && i.fn !== null) {
        if (l) {
          i.f ^= T;
        } else if ((s & Ne) !== 0) {
          n.push(i);
        } else if (et(i)) {
          if ((s & G) !== 0) {
            this.add_dirty_reaction(i, B);
          }
          Ie(i);
        }
        const u = i.first;
        if (u !== null) {
          i = u;
          continue;
        }
      }
      while (i !== null) {
        const a = i.next;
        if (a !== null) {
          i = a;
          break;
        }
        i = i.parent;
      }
    }
  }
  #E() {
    for (let t = this.prev; t !== null; t = t.prev) {
      if (!t.is_fork && this.#m(t)) {
        return t;
      }
    }
    return null;
  }
  #m(t) {
    for (const [n, [, r]] of this.current) {
      if (!r && t.current.has(n)) {
        return true;
      }
    }
    return false;
  }
  #c(t) {
    for (const [r, i] of t.current) {
      if (!this.previous.has(r) && t.previous.has(r)) {
        this.previous.set(r, t.previous.get(r));
      }
      this.current.set(r, i);
    }
    for (const [r, i] of t.async_deriveds) {
      const s = this.async_deriveds.get(r);
      if (s) {
        i.promise.then(s.resolve).catch(s.reject);
      }
    }
    t.async_deriveds.clear();
    this.transfer_reactions(t.#i);
    const n = (r) => {
      const i = r.reactions;
      if (i !== null && ((r.f & M) === 0 || (r.f & (k | B)) !== 0)) {
        for (const f of i) {
          const s = f.f;
          if ((s & M) !== 0) {
            n(f);
          } else {
            const l = f;
            if (s & (Oe | G) && !this.async_deriveds.has(l)) {
              this.#i.delete(l);
              x(l, k);
              this.schedule(l);
            }
          }
        }
      }
    };
    for (const r of this.current.keys()) {
      n(r);
    }
    this.oncommit(() => t.discard());
    t.#d();
    m = this;
    this.#_();
  }
  #p(t) {
    for (let n = 0; n < t.length; n += 1) {
      gn(t[n], this.#i);
    }
  }
  capture(t, n, r = false) {
    if (t.v !== S && !this.previous.has(t)) {
      this.previous.set(t, t.v);
    }
    if ((t.f & de) === 0) {
      this.current.set(t, [n, r]);
      K?.set(t, n);
    }
    if (!this.is_fork) {
      t.v = n;
    }
  }
  activate() {
    m = this;
  }
  deactivate() {
    m = null;
    K = null;
  }
  flush() {
    try {
      gt = true;
      m = this;
      this.#_();
    } finally {
      Wt = 0;
      Tt = null;
      Ue = null;
      it = null;
      gt = false;
      m = null;
      K = null;
      te.clear();
    }
  }
  discard() {
    for (const t of this.#o) {
      t(this);
    }
    this.#o.clear();
    for (const t of this.async_deriveds.values()) {
      t.reject(qe);
    }
    this.#d();
    this.#n?.resolve();
  }
  register_created_effect(t) {
    this.#s.push(t);
  }
  #b() {
    for (let c = pt; c !== null; c = c.next) {
      const t = c.id < this.id;
      const n = [];
      for (const [d, [h, v]] of this.current) {
        if (c.current.has(d)) {
          const r = c.current.get(d)[0];
          if (t && h !== r) {
            c.current.set(d, [h, v]);
          } else {
            continue;
          }
        }
        n.push(d);
      }
      if (t) {
        for (const [d, h] of this.async_deriveds) {
          const v = c.async_deriveds.get(d);
          if (v) {
            h.promise.then(v.resolve).catch(v.reject);
          }
        }
      }
      const i = [...c.current.keys()].filter((d) => !c.current.get(d)[1]);
      if (!!c.started && i.length !== 0) {
        const s = i.filter((d) => !this.current.has(d));
        if (s.length === 0) {
          if (t) {
            c.discard();
          }
        } else if (n.length > 0) {
          if (t) {
            for (const d of this.#h) {
              c.unskip_effect(d, (h) => {
                if ((h.f & (G | Oe)) !== 0) {
                  c.schedule(h);
                } else {
                  c.#p([h]);
                }
              });
            }
          }
          c.activate();
          const l = new Set();
          let f = new Map();
          for (const o of n) {
            En(o, s, l, f);
          }
          f = new Map();
          const u = [...c.current].filter(([d, h]) => {
            const v = this.current.get(d);
            if (v) {
              return v[0] !== h[0] || v[1] !== h[1];
            } else {
              return true;
            }
          }).map(([d]) => d);
          if (u.length > 0) {
            for (const d of this.#s) {
              if ((d.f & (O | F | ft)) === 0 && $t(d, u, f)) {
                if ((d.f & (Oe | G)) !== 0) {
                  x(d, k);
                  c.schedule(d);
                } else {
                  c.add_dirty_reaction(d, k);
                }
              }
            }
          }
          if (c.#r.length > 0 && !c.#u) {
            c.apply();
            for (const a of c.#a()) {
              c.#w(a, [], []);
            }
          }
          c.deactivate();
        }
      }
    }
  }
  increment(t, n) {
    this.#e += 1;
    if (t) {
      const r = this.#l.get(n) ?? 0;
      this.#l.set(n, r + 1);
    }
  }
  decrement(t, n) {
    this.#e -= 1;
    if (t) {
      const r = this.#l.get(n) ?? 0;
      if (r === 1) {
        this.#l.delete(n);
      } else {
        this.#l.set(n, r - 1);
      }
    }
    if (!this.#u) {
      this.#u = true;
      ae(() => {
        this.#u = false;
        if (this.linked) {
          this.flush();
        }
      });
    }
  }
  transfer_reactions(t) {
    for (const [n, r] of t) {
      this.add_dirty_reaction(n, r);
    }
    t.clear();
  }
  add_dirty_reaction(t, n) {
    if (n === k || this.#i.get(t) !== k) {
      this.#i.set(t, n);
    }
  }
  oncommit(t) {
    this.#t.add(t);
  }
  ondiscard(t) {
    this.#o.add(t);
  }
  settled() {
    return (this.#n ??= fn()).promise;
  }
  static ensure() {
    if (m === null) {
      const t = m = new _e();
      if (!gt && !Ye) {
        ae(() => {
          if (!t.started) {
            t.flush();
          }
        });
      }
    }
    return m;
  }
  apply() {
    {
      K = null;
      return;
    }
  }
  schedule(t) {
    Tt = t;
    if (t.b?.is_pending && (t.f & (Ne | ht | on)) !== 0 && (t.f & Le) === 0) {
      t.b.defer_effect(t);
      return;
    }
    this.#r.push(t);
  }
  #d() {
    if (this.linked) {
      const t = this.prev;
      const n = this.next;
      if (t === null) {
        pt = n;
      } else {
        t.next = n;
      }
      if (n === null) {
        Re = t;
      } else {
        n.prev = t;
      }
      this.linked = false;
    }
  }
}
function Mr(e) {
  const t = Ye;
  const n = Me;
  Me = null;
  Ye = true;
  try {
    var r;
    while (true) {
      Er();
      if (m === null) {
        return r;
      }
      m.flush();
    }
  } finally {
    Ye = t;
    Me = n;
  }
}
function Nr() {
  try {
    gr();
  } catch (e) {
    Q(e, Tt);
  }
}
let le = null;
function Xt(e) {
  const t = e.length;
  if (t !== 0) {
    for (let n = 0; n < t;) {
      const r = e[n++];
      if ((r.f & (O | F)) === 0 && et(r) && (le = new Set(), Ie(r), r.deps === null && r.first === null && r.nodes === null && r.teardown === null && r.ac === null && Pn(r), le?.size > 0)) {
        te.clear();
        for (const i of le) {
          if ((i.f & (O | F)) !== 0) {
            continue;
          }
          const s = [i];
          let l = i.parent;
          while (l !== null) {
            if (le.has(l)) {
              le.delete(l);
              s.push(l);
            }
            l = l.parent;
          }
          for (let f = s.length - 1; f >= 0; f--) {
            const o = s[f];
            if ((o.f & (O | F)) === 0) {
              Ie(o);
            }
          }
        }
        le.clear();
      }
    }
    le = null;
  }
}
function En(e, t, n, r) {
  if (!n.has(e) && (n.add(e), e.reactions !== null)) {
    for (const i of e.reactions) {
      const s = i.f;
      if ((s & M) !== 0) {
        En(i, t, n, r);
      } else if ((s & (Oe | G)) !== 0 && (s & k) === 0 && $t(i, t, r)) {
        x(i, k);
        jt(i);
      }
    }
  }
}
function $t(e, t, n) {
  const r = n.get(e);
  if (r !== undefined) {
    return r;
  }
  if (e.deps !== null) {
    for (const i of e.deps) {
      if (Ke.call(t, i)) {
        return true;
      }
      if ((i.f & M) !== 0 && $t(i, t, n)) {
        n.set(i, true);
        return true;
      }
    }
  }
  n.set(e, false);
  return false;
}
function jt(e) {
  m.schedule(e);
}
function kn(e, t) {
  if ((e.f & re) === 0 || (e.f & T) === 0) {
    if ((e.f & k) !== 0) {
      t.d.push(e);
    } else if ((e.f & B) !== 0) {
      t.m.push(e);
    }
    x(e, T);
    for (let n = e.first; n !== null;) {
      kn(n, t);
      n = n.next;
    }
  }
}
function xn(e) {
  x(e, T);
  for (let t = e.first; t !== null;) {
    xn(t);
    t = t.next;
  }
}
let at = new Set();
const te = new Map();
let Sn = false;
function Se(e, t) {
  const n = {
    f: 0,
    v: e,
    reactions: null,
    equals: dn,
    rv: 0,
    wv: 0
  };
  return n;
}
function N(e, t) {
  const n = Se(e);
  Ln(n);
  return n;
}
function Dr(e, t = false, n = true) {
  const r = Se(e);
  if (!t) {
    r.equals = hn;
  }
  return r;
}
function D(e, t, n = false) {
  if (w !== null && (!W || (w.f & ft) !== 0) && _n() && (w.f & (M | G | Oe | ft)) !== 0 && (ne === null || !ne.has(e))) {
    br();
  }
  const r = n ? ce(t) : t;
  return Fe(e, r, it);
}
var we = null;
var At = 0;
function Fe(e, t, n = null) {
  if (!e.equals(t)) {
    if (Te) {
      te.set(e, t);
    } else if (!te.has(e)) {
      te.set(e, e.v);
    }
    const r = _e.ensure();
    r.capture(e, t);
    if ((e.f & M) !== 0) {
      const i = e;
      if ((e.f & k) !== 0) {
        Lt(i);
      }
      if (K === null) {
        Pt(i);
      }
    }
    e.wv = jn();
    we = null;
    At = 0;
    Tn(e, k, n);
    we = null;
    if (y !== null && (y.f & T) !== 0 && (y.f & (re | ve)) === 0) {
      if (q === null) {
        Gr([e]);
      } else {
        q.push(e);
      }
    }
    if (!r.is_fork && at.size > 0 && !Sn) {
      Pr();
    }
  }
  return t;
}
function Pr() {
  Sn = false;
  for (const e of at) {
    if ((e.f & T) !== 0) {
      x(e, B);
    }
    let t;
    try {
      t = et(e);
    } catch {
      t = true;
    }
    if (t) {
      Ie(e);
    }
  }
  at.clear();
}
function Fr(e, t = 1) {
  let n = g(e);
  const r = t === 1 ? n++ : n--;
  D(e, n);
  return r;
}
function Ge(e) {
  D(e, e.v + 1);
}
function Tn(e, t, n) {
  const r = e.reactions;
  if (r !== null) {
    const i = r.length;
    At += i;
    if (At > 1e5 && we === null) {
      we = new Set();
    }
    if (we !== null) {
      if (we.has(e)) {
        return;
      }
      we.add(e);
    }
    for (let s = 0; s < i; s++) {
      const l = r[s];
      const f = l.f;
      const o = (f & k) === 0;
      if (o) {
        x(l, t);
      }
      if ((f & ft) !== 0) {
        at.add(l);
      } else if ((f & M) !== 0) {
        const u = l;
        K?.delete(u);
        Tn(u, B, n);
      } else if (o) {
        const a = l;
        if ((f & G) !== 0 && le !== null) {
          le.add(a);
        }
        if (n !== null) {
          n.push(a);
        } else {
          jt(a);
        }
      }
    }
  }
}
function ce(e) {
  if (typeof e != "object" || e === null || nt in e || un in e) {
    return e;
  }
  const t = Jn(e);
  if (t !== Xn && t !== Zn) {
    return e;
  }
  const n = new Map();
  const r = sn(e);
  const i = N(0);
  const s = xe;
  const l = (f) => {
    if (xe === s) {
      return f();
    }
    const o = w;
    const u = xe;
    z(null);
    Qt(s);
    const a = f();
    z(o);
    Qt(u);
    return a;
  };
  if (r) {
    n.set("length", N(e.length));
  }
  return new Proxy(e, {
    defineProperty(f, o, u) {
      if (!("value" in u) || u.configurable === false || u.enumerable === false || u.writable === false) {
        mr();
      }
      const a = n.get(o);
      if (a === undefined) {
        l(() => {
          const c = N(u.value);
          n.set(o, c);
          return c;
        });
      } else {
        D(a, u.value, true);
      }
      return true;
    },
    deleteProperty(f, o) {
      const u = n.get(o);
      if (u === undefined) {
        if (o in f) {
          const a = l(() => N(S));
          n.set(o, a);
          Ge(i);
        }
      } else {
        D(u, S);
        Ge(i);
      }
      return true;
    },
    get(f, o, u) {
      if (o === nt) {
        return e;
      }
      let a = n.get(o);
      const c = o in f;
      if (a === undefined && (!c || Ee(f, o)?.writable)) {
        a = l(() => {
          const h = ce(c ? f[o] : S);
          const v = N(h);
          return v;
        });
        n.set(o, a);
      }
      if (a !== undefined) {
        const d = g(a);
        if (d === S) {
          return undefined;
        } else {
          return d;
        }
      }
      return Reflect.get(f, o, u);
    },
    getOwnPropertyDescriptor(f, o) {
      this.has?.(f, o);
      const u = Reflect.getOwnPropertyDescriptor(f, o);
      const a = n.get(o);
      if (a !== undefined) {
        const c = g(a);
        if (c === S) {
          return;
        }
        if (u && "value" in u) {
          u.value = c;
        } else {
          return {
            enumerable: true,
            configurable: true,
            value: c,
            writable: true
          };
        }
      }
      return u;
    },
    has(f, o) {
      if (o === nt) {
        return true;
      }
      let u = n.get(o);
      const a = u !== undefined && u.v !== S || Reflect.has(f, o);
      if (u !== undefined || (y !== null || w !== null) && (!a || Ee(f, o)?.writable)) {
        if (u === undefined) {
          u = l(() => {
            const d = a ? ce(f[o]) : S;
            const h = N(d);
            return h;
          });
          n.set(o, u);
        }
        const c = g(u);
        if (c === S) {
          return false;
        }
      }
      return a;
    },
    set(f, o, u, a) {
      let c = n.get(o);
      let d = o in f;
      if (r && o === "length") {
        for (let h = u; h < c.v; h += 1) {
          let v = n.get(h + "");
          if (v !== undefined) {
            D(v, S);
          } else if (h in f) {
            v = l(() => N(S));
            n.set(h + "", v);
          }
        }
      }
      if (c === undefined) {
        if (!d || Ee(f, o)?.writable) {
          c = l(() => N(undefined));
          D(c, ce(u));
          n.set(o, c);
        }
      } else {
        d = c.v !== S;
        const b = l(() => ce(u));
        D(c, b);
      }
      const _ = Reflect.getOwnPropertyDescriptor(f, o);
      if (_?.set) {
        _.set.call(a, u);
      }
      if (!d) {
        if (r && typeof o == "string") {
          const p = n.get("length");
          const E = Number(o);
          if (Number.isInteger(E) && E >= p.v) {
            D(p, E + 1);
          }
        }
        Ge(i);
      }
      return true;
    },
    ownKeys(f) {
      g(i);
      const o = Reflect.ownKeys(f).filter((c) => {
        const d = n.get(c);
        return d === undefined || d.v !== S;
      });
      for (var [u, a] of n) {
        if (a.v !== S && !(u in f)) {
          o.push(u);
        }
      }
      return o;
    },
    setPrototypeOf() {
      wr();
    }
  });
}
var Zt;
var An;
var Cn;
var Rn;
function Ir() {
  if (Zt === undefined) {
    Zt = window;
    An = /Firefox/.test(navigator.userAgent);
    const e = Element.prototype;
    const t = Node.prototype;
    const n = Text.prototype;
    Cn = Ee(t, "firstChild").get;
    Rn = Ee(t, "nextSibling").get;
    if (Ut(e)) {
      e[kt] = undefined;
      e[ir] = null;
      e[sr] = undefined;
      e.__e = undefined;
    }
    if (Ut(n)) {
      n[xt] = undefined;
    }
  }
}
function he(e = "") {
  return document.createTextNode(e);
}
function We(e) {
  return Cn.call(e);
}
function Qe(e) {
  return Rn.call(e);
}
function On(e, t) {
  return We(e);
}
function je(e, t = false) {
  {
    const n = We(e);
    if (n instanceof Comment && n.data === "") {
      return Qe(n);
    } else {
      return n;
    }
  }
}
function pe(e, t = false) {
  return We(e);
}
function P(e, t = 1, n = false) {
  let r = e;
  while (t--) {
    r = Qe(r);
  }
  return r;
}
function Lr(e) {
  e.textContent = "";
}
function Mn() {
  return false;
}
function Nn(e, t, n) {
  if (n) {
    return document.createElement(e, { is: n });
  } else {
    return document.createElement(e);
  }
}
function $r(e) {
  const t = y;
  if (t === null) {
    w.f |= de;
    return e;
  }
  if ((t.f & Le) === 0 && (t.f & Ne) === 0) {
    throw e;
  }
  Q(e, t);
}
function Q(e, t) {
  if (e === cn) {
    throw e;
  }
  if (t === null || (t.f & O) === 0) {
    while (t !== null) {
      if ((t.f & Et) !== 0 && (t.f & (O | lt)) === 0) {
        if ((t.f & Le) === 0) {
          throw e;
        }
        try {
          t.b.error(e);
          return;
        } catch (n) {
          e = n;
        }
      }
      t = t.parent;
    }
    throw e;
  }
}
function jr(e, t) {
  const n = t.last;
  if (n === null) {
    t.last = t.first = e;
  } else {
    n.next = e;
    e.prev = n;
    t.last = e;
  }
}
function fe(e, t) {
  const n = y;
  if (n !== null && (n.f & F) !== 0) {
    e |= F;
  }
  const r = {
    ctx: Z,
    deps: null,
    nodes: null,
    f: e | k | X,
    first: null,
    fn: t,
    last: null,
    next: null,
    parent: n,
    b: n && n.b,
    prev: null,
    teardown: null,
    wv: 0,
    ac: null
  };
  m?.register_created_effect(r);
  let i = r;
  if ((e & Ne) !== 0) {
    if (Ue !== null) {
      Ue.push(r);
    } else {
      _e.ensure().schedule(r);
    }
  } else if (t !== null) {
    try {
      Ie(r);
    } catch (l) {
      L(r);
      throw l;
    }
    if (i.deps === null && i.teardown === null && i.nodes === null && i.first === i.last && (i.f & $e) === 0) {
      i = i.first;
      if ((e & G) !== 0 && (e & De) !== 0 && i !== null) {
        i.f |= De;
      }
    }
  }
  if (i !== null && (i.parent = n, n !== null && jr(i, n), w !== null && (w.f & M) !== 0 && (e & ve) === 0)) {
    const s = w;
    (s.effects ??= []).push(i);
  }
  return r;
}
function Bt() {
  return w !== null && !W;
}
function Br(e) {
  const t = fe(ht, null);
  x(t, T);
  t.teardown = e;
  return t;
}
function Hr(e) {
  return fe(Ne | nr, e);
}
function qr(e) {
  _e.ensure();
  const t = fe(ve | $e, e);
  return (n = {}) => new Promise((r) => {
    if (n.outro) {
      ke(t, () => {
        L(t);
        r(undefined);
      });
    } else {
      L(t);
      r(undefined);
    }
  });
}
function Vr(e) {
  return fe(Ne, e);
}
function zr(e) {
  return fe(Oe | $e, e);
}
function Ht(e, t = 0) {
  return fe(ht | t, e);
}
function ie(e, t = [], n = [], r = []) {
  Sr(r, t, n, (i) => {
    fe(ht, () => {
      e(...i.map(g));
    });
  });
}
function qt(e, t = 0) {
  const n = fe(G | t, e);
  return n;
}
function V(e) {
  return fe(re | $e, e);
}
function Dn(e) {
  const t = e.teardown;
  if (t !== null) {
    const n = Te;
    const r = w;
    Jt(true);
    z(null);
    try {
      t.call(null);
    } catch (i) {
      Q(i, e.parent);
    } finally {
      Jt(n);
      z(r);
    }
  }
}
function Vt(e, t = false) {
  let n = e.first;
  for (e.first = e.last = null; n !== null;) {
    const i = n.ac;
    if (i !== null) {
      Je(() => {
        i.abort(Ze);
      });
    }
    const r = n.next;
    if ((n.f & ve) !== 0) {
      n.parent = null;
    } else {
      L(n, t);
    }
    n = r;
  }
}
function Yr(e) {
  for (let t = e.first; t !== null;) {
    const n = t.next;
    if ((t.f & re) === 0) {
      L(t);
    }
    t = n;
  }
}
function L(e, t = true) {
  let n = false;
  if ((t || (e.f & tr) !== 0) && e.nodes !== null && e.nodes.end !== null) {
    Ur(e.nodes.start, e.nodes.end);
    n = true;
  }
  e.f |= lt;
  Vt(e, t && !n);
  Xe(e, 0);
  const r = e.nodes && e.nodes.t;
  if (r !== null) {
    for (const s of r) {
      s.stop();
    }
  }
  Dn(e);
  e.f ^= lt;
  e.f |= O;
  const i = e.parent;
  if (i !== null && i.first !== null) {
    Pn(e);
  }
  e.next = e.prev = e.teardown = e.ctx = e.deps = e.fn = e.nodes = e.ac = e.b = null;
}
function Ur(e, t) {
  while (e !== null) {
    const n = e === t ? null : Qe(e);
    e.remove();
    e = n;
  }
}
function Pn(e) {
  const { parent: t, prev: n, next: r } = e;
  if (n !== null) {
    n.next = r;
  }
  if (r !== null) {
    r.prev = n;
  }
  if (t !== null) {
    if (t.first === e) {
      t.first = r;
    }
    if (t.last === e) {
      t.last = n;
    }
  }
}
function ke(e, t, n = true) {
  const r = [];
  e.f |= Dt;
  Fn(e, r, true);
  const i = () => {
    if (n) {
      L(e);
    }
    if (t) {
      t();
    }
  };
  let s = r.length;
  if (s > 0) {
    const l = () => --s || i();
    for (const f of r) {
      f.out(l);
    }
  } else {
    i();
  }
}
function Fn(e, t, n) {
  if ((e.f & F) === 0) {
    e.f ^= F;
    const r = e.nodes && e.nodes.t;
    if (r !== null) {
      for (const f of r) {
        if (f.is_global || n) {
          t.push(f);
        }
      }
    }
    for (let i = e.first; i !== null;) {
      const s = i.next;
      if ((i.f & ve) === 0) {
        const l = (i.f & De) !== 0 || (i.f & re) !== 0 && (e.f & G) !== 0;
        Fn(i, t, l ? n : false);
      }
      i = s;
    }
  }
}
function ct(e) {
  e.f &= ~Dt;
  In(e, true);
}
function In(e, t) {
  if ((e.f & Dt) === 0 && (e.f & F) !== 0) {
    e.f ^= F;
    if ((e.f & T) === 0) {
      x(e, k);
      _e.ensure().schedule(e);
    }
    for (let n = e.first; n !== null;) {
      const r = n.next;
      const i = (n.f & De) !== 0 || (n.f & re) !== 0;
      In(n, i ? t : false);
      n = r;
    }
    const s = e.nodes && e.nodes.t;
    if (s !== null) {
      for (const l of s) {
        if (l.is_global || t) {
          l.in();
        }
      }
    }
  }
}
function zt(e, t) {
  if (e.nodes) {
    for (let n = e.nodes.start, r = e.nodes.end; n !== null;) {
      const i = n === r ? null : Qe(n);
      t.append(n);
      n = i;
    }
  }
}
let Te = false;
function Jt(e) {
  Te = e;
}
let w = null;
let W = false;
function z(e) {
  w = e;
}
let y = null;
function se(e) {
  y = e;
}
let ne = null;
function Ln(e) {
  if (w !== null && ((w.f & ot) !== 0 || (w.f & M) !== 0)) {
    (ne ??= new Set()).add(e);
  }
}
let I = null;
let j = 0;
let q = null;
function Gr(e) {
  q = e;
}
let $n = 1;
let ye = 0;
let xe = ye;
function Qt(e) {
  xe = e;
}
function jn() {
  return ++$n;
}
function et(e) {
  const t = e.f;
  if ((t & k) !== 0) {
    return true;
  }
  if ((t & B) !== 0) {
    const n = e.deps;
    for (let r = n.length, i = 0; i < r; i++) {
      const s = n[i];
      if (et(s)) {
        bn(s);
      }
      if (s.wv > e.wv) {
        return true;
      }
    }
    if ((t & X) !== 0 && K === null) {
      x(e, T);
    }
  }
  return false;
}
function Bn(e, t, n = true) {
  const r = e.reactions;
  if (r !== null && (ne === null || !ne.has(e))) {
    for (let i = 0; i < r.length; i++) {
      const s = r[i];
      if ((s.f & M) !== 0) {
        Bn(s, t, false);
      } else if (t === s) {
        if (n) {
          x(s, k);
        } else if ((s.f & T) !== 0) {
          x(s, B);
        }
        jt(s);
      }
    }
  }
}
function Hn(e) {
  const t = I;
  const n = j;
  let r = q;
  const i = w;
  const s = ne;
  const l = Z;
  const f = W;
  const o = xe;
  const u = e.f;
  I = null;
  j = 0;
  q = null;
  w = (u & (re | ve)) === 0 ? e : null;
  ne = null;
  Pe(e.ctx);
  W = false;
  xe = ++ye;
  if (e.ac !== null) {
    Je(() => {
      e.ac.abort(Ze);
    });
    e.ac = null;
  }
  try {
    e.f |= ot;
    const a = e.fn;
    const c = a();
    e.f |= Le;
    const d = en(e);
    if (_n() && q !== null && !W && d !== null && (e.f & (M | B | k)) === 0) {
      for (let h = 0; h < q.length; h++) {
        Bn(q[h], e);
      }
    }
    if (i !== null && i !== e) {
      ye++;
      if (i.deps !== null) {
        for (let v = 0; v < n; v += 1) {
          i.deps[v].rv = ye;
        }
      }
      if (t !== null) {
        for (const v of t) {
          v.rv = ye;
        }
      }
      if (q !== null) {
        if (r === null) {
          r = q;
        } else {
          r.push(...q);
        }
      }
    }
    if ((e.f & de) !== 0) {
      e.f ^= de;
    }
    return c;
  } catch (v) {
    en(e);
    return $r(v);
  } finally {
    e.f ^= ot;
    I = t;
    j = n;
    q = r;
    w = i;
    ne = s;
    Pe(l);
    W = f;
    xe = o;
  }
}
function en(e) {
  let t = e.deps;
  const n = m?.is_fork;
  if (I !== null) {
    var r;
    if (!n) {
      Xe(e, j);
    }
    if (t !== null && j > 0) {
      t.length = j + I.length;
      r = 0;
      for (; r < I.length; r++) {
        t[j + r] = I[r];
      }
    } else {
      e.deps = t = I;
    }
    if (Bt() && (e.f & X) !== 0) {
      for (r = j; r < t.length; r++) {
        (t[r].reactions ??= []).push(e);
      }
    }
  } else if (!n && t !== null && j < t.length) {
    Xe(e, j);
    t.length = j;
  }
  return t;
}
function Kr(e, t) {
  let n = t.reactions;
  if (n !== null) {
    const r = Wn.call(n, e);
    if (r !== -1) {
      const i = n.length - 1;
      if (i === 0) {
        n = t.reactions = null;
      } else {
        n[r] = n[i];
        n.pop();
      }
    }
  }
  if (n === null && (t.f & M) !== 0 && (I === null || !Ke.call(I, t))) {
    var s = t;
    if ((s.f & X) !== 0) {
      s.f ^= X;
    }
    if (s.v !== S) {
      Pt(s);
    }
    if (s.ac !== null) {
      Je(() => {
        s.ac.abort(Ze);
        s.ac = null;
        x(s, k);
      });
    }
    Rr(s);
    Xe(s, 0);
  }
}
function Xe(e, t) {
  const n = e.deps;
  if (n !== null) {
    for (let r = t; r < n.length; r++) {
      Kr(e, n[r]);
    }
  }
}
function Ie(e) {
  const t = e.f;
  if ((t & O) === 0) {
    x(e, T);
    const n = y;
    y = e;
    try {
      if ((t & (G | on)) !== 0) {
        Yr(e);
      } else {
        Vt(e);
      }
      Dn(e);
      const r = Hn(e);
      e.teardown = typeof r == "function" ? r : null;
      e.wv = $n;
      var i;
    } finally {
      y = n;
    }
  }
}
async function Wr() {
  await Promise.resolve();
  Mr();
}
function g(e) {
  const t = e.f;
  const n = (t & M) !== 0;
  if (w !== null && !W) {
    const r = y !== null && (y.f & O) !== 0;
    if (!r && (ne === null || !ne.has(e))) {
      const i = w.deps;
      if ((w.f & ot) !== 0) {
        if (e.rv < ye) {
          e.rv = ye;
          if (I === null && i !== null && i[j] === e) {
            j++;
          } else if (I === null) {
            I = [e];
          } else {
            I.push(e);
          }
        }
      } else {
        w.deps ??= [];
        if (!Ke.call(w.deps, e)) {
          w.deps.push(e);
        }
        const s = e.reactions;
        if (s === null) {
          e.reactions = [w];
        } else if (!Ke.call(s, w)) {
          s.push(w);
        }
      }
    }
  }
  if (Te && te.has(e)) {
    return te.get(e);
  }
  if (n) {
    const l = e;
    if (Te) {
      let f = l.v;
      if ((l.f & T) === 0 && l.reactions !== null || Vn(l)) {
        f = Lt(l);
      }
      te.set(l, f);
      return f;
    }
    const o = (l.f & X) === 0 && !W && w !== null && (w.f & X) !== 0;
    const u = (l.f & Le) === 0;
    if (et(l)) {
      if (o) {
        l.f |= X;
      }
      bn(l);
    }
    if (o && !u) {
      yn(l);
      qn(l);
    }
  }
  if (K?.has(e)) {
    return K.get(e);
  }
  if ((e.f & de) !== 0) {
    throw e.v;
  }
  return e.v;
}
function qn(e) {
  e.f |= X;
  if (e.deps !== null) {
    for (const n of e.deps) {
      const t = n.reactions;
      if (t === null) {
        n.reactions = [e];
      } else if (!Ke.call(t, e)) {
        t.push(e);
      }
      if ((n.f & M) !== 0 && (n.f & X) === 0) {
        yn(n);
        qn(n);
      }
    }
  }
}
function Vn(e) {
  if (e.v === S) {
    return true;
  }
  if (e.deps === null) {
    return false;
  }
  for (const t of e.deps) {
    if (te.has(t) || (t.f & M) !== 0 && Vn(t)) {
      return true;
    }
  }
  return false;
}
function Yt(e) {
  const t = W;
  try {
    W = true;
    return e();
  } finally {
    W = t;
  }
}
const Xr = ["touchstart", "touchmove"];
function Zr(e) {
  return Xr.includes(e);
}
const Ve = Symbol("events");
const zn = new Set();
const Ct = new Set();
function ge(e, t, n) {
  (t[Ve] ??= {})[e] = n;
}
function Ae(e) {
  for (let t = 0; t < e.length; t++) {
    zn.add(e[t]);
  }
  for (const n of Ct) {
    n(e);
  }
}
let mt = null;
let wt = false;
function tn(e) {
  const t = this;
  const n = t.ownerDocument;
  const r = e.type;
  const i = e.composedPath?.() || [];
  let s = i[0] || e.target;
  mt = e;
  if (!wt) {
    wt = true;
    setTimeout(() => {
      wt = false;
      mt = null;
    });
  }
  let l = 0;
  const f = mt === e && e[Ve];
  if (f) {
    const o = i.indexOf(f);
    if (o !== -1 && (t === document || t === window)) {
      e[Ve] = t;
      return;
    }
    const u = i.indexOf(t);
    if (u === -1) {
      return;
    }
    if (o <= u) {
      l = o;
    }
  }
  s = i[l] || e.target;
  if (s !== t) {
    ln(e, "currentTarget", {
      configurable: true,
      get() {
        return s || n;
      }
    });
    const a = w;
    const c = y;
    z(null);
    se(null);
    try {
      var d;
      const h = [];
      while (s !== null && s !== t) {
        try {
          const v = s[Ve]?.[r];
          if (v != null && (!s.disabled || e.target === s)) {
            v.call(s, e);
          }
        } catch (b) {
          if (d) {
            h.push(b);
          } else {
            d = b;
          }
        }
        if (e.cancelBubble) {
          break;
        }
        l++;
        s = l < i.length ? i[l] : null;
      }
      if (d) {
        for (let b of h) {
          queueMicrotask(() => {
            throw b;
          });
        }
        throw d;
      }
    } finally {
      e[Ve] = t;
      delete e.currentTarget;
      z(a);
      se(c);
    }
  }
}
const Jr = globalThis?.window?.trustedTypes && globalThis.window.trustedTypes.createPolicy("svelte-trusted-html", { createHTML: (e) => e });
function Qr(e) {
  return Jr?.createHTML(e) ?? e;
}
function ei(e) {
  const t = Nn("template");
  t.innerHTML = Qr(e.replaceAll("<!>", "<!---->"));
  return t.content;
}
function Rt(e, t) {
  const n = y;
  if (n.nodes === null) {
    n.nodes = {
      start: e,
      end: t,
      a: null,
      t: null
    };
  }
}
function Y(e, t) {
  const n = (t & ar) !== 0;
  const r = (t & cr) !== 0;
  const s = !e.startsWith("<!>");
  let i;
  return () => {
    if (i === undefined) {
      i = ei(s ? e : "<!>" + e);
      if (!n) {
        i = We(i);
      }
    }
    const l = r || An ? document.importNode(i, true) : i.cloneNode(true);
    if (n) {
      const f = We(l);
      const o = l.lastChild;
      Rt(f, o);
    } else {
      Rt(l, l);
    }
    return l;
  };
}
function ti(e = "") {
  {
    const t = he(e + "");
    Rt(t, t);
    return t;
  }
}
function H(e, t) {
  if (e !== null) {
    e.before(t);
  }
}
function ni(e) {
  let t = 0;
  const n = Se(0);
  let r;
  return () => {
    if (Bt()) {
      g(n);
      Ht(() => {
        if (t === 0) {
          r = Yt(() => e(() => Ge(n)));
        }
        t += 1;
        return () => {
          ae(() => {
            t -= 1;
            if (t === 0) {
              r?.();
              r = undefined;
              Ge(n);
            }
          });
        };
      });
    }
  };
}
var ri = De | $e;
function Yn(e, t, n, r) {
  new ii(e, t, n, r);
}
let ii = class {
  parent;
  is_pending = false;
  transform_error;
  #t;
  #o = null;
  #e;
  #l;
  #n;
  #r = null;
  #s = null;
  #i = null;
  #f = null;
  #h = 0;
  #u = 0;
  #v = false;
  #g = new Map();
  #a = null;
  #_ = ni(() => {
    this.#a = Se(this.#h);
    return () => {
      this.#a = null;
    };
  });
  constructor(t, n, r, i) {
    this.#t = t;
    this.#e = n;
    this.#l = (s) => {
      const l = y;
      l.b = this;
      l.f |= Et;
      r(s);
    };
    this.parent = y.b;
    this.transform_error = i ?? this.parent?.transform_error ?? ((s) => s);
    this.#n = qt(() => {
      this.#b();
    }, ri);
  }
  #w() {
    try {
      this.#r = V(() => this.#l(this.#t));
    } catch (t) {
      this.error(t);
    }
  }
  #E(t) {
    const n = this.#e.failed;
    const { reset: r, invoke_onerror: i } = this.#m(t);
    ae(i);
    if (n) {
      this.#i = V(() => {
        n(this.#t, () => t, () => r);
      });
    }
  }
  #m(t) {
    let n = false;
    let r = false;
    const i = () => {
      if (!this.#c()) {
        if (n) {
          hr();
          return;
        }
        n = true;
        if (r) {
          yr();
        }
        if (this.#i !== null) {
          ke(this.#i, () => {
            this.#i = null;
          });
        }
        this.#y(() => {
          this.#b();
        });
      }
    };
    return {
      reset: i,
      invoke_onerror: () => {
        if (!this.#c()) {
          try {
            r = true;
            this.#e.onerror?.(t, i);
            r = false;
          } catch (l) {
            Q(l, this.#n && this.#n.parent);
          }
        }
      }
    };
  }
  #c() {
    return (this.#n.f & (O | lt)) !== 0;
  }
  #p() {
    const t = this.#e.pending;
    if (t) {
      this.is_pending = true;
      this.#s = V(() => t(this.#t));
      ae(() => {
        if (!this.#c()) {
          const n = this.#f = document.createDocumentFragment();
          var r = he();
          var i = false;
          n.append(r);
          this.#r = this.#y(() => {
            try {
              return V(() => this.#l(r));
            } catch (s) {
              try {
                this.error(s);
                i = true;
              } catch (l) {
                Q(l, this.#n.parent);
              }
              return null;
            }
          });
          if (this.#r === null) {
            this.#f = null;
            if (i) {
              this.#d(m);
            }
            return;
          }
          if (this.#u === 0) {
            this.#t.before(n);
            this.#f = null;
            ke(this.#s, () => {
              this.#s = null;
            });
            this.#d(m);
          }
        }
      });
    }
  }
  #b() {
    try {
      this.is_pending = this.has_pending_snippet();
      this.#u = 0;
      this.#h = 0;
      this.#r = V(() => {
        this.#l(this.#t);
      });
      if (this.#u > 0) {
        const t = this.#f = document.createDocumentFragment();
        zt(this.#r, t);
        const n = this.#e.pending;
        this.#s = V(() => n(this.#t));
      } else {
        this.#d(m);
      }
    } catch (n) {
      this.error(n);
    }
  }
  #d(t) {
    this.is_pending = false;
    t.transfer_reactions(this.#g);
  }
  defer_effect(t) {
    gn(t, this.#g);
  }
  is_rendered() {
    return !this.is_pending && (!this.parent || this.parent.is_rendered());
  }
  has_pending_snippet() {
    return !!this.#e.pending;
  }
  #y(t) {
    const n = y;
    const r = w;
    const i = Z;
    se(this.#n);
    z(this.#n);
    Pe(this.#n.ctx);
    try {
      _e.ensure();
      return t();
    } finally {
      se(n);
      z(r);
      Pe(i);
    }
  }
  #k(t, n) {
    if (!this.has_pending_snippet()) {
      if (this.parent) {
        this.parent.#k(t, n);
      }
      return;
    }
    this.#u += t;
    if (this.#u === 0) {
      this.#d(n);
      if (this.#s) {
        ke(this.#s, () => {
          this.#s = null;
        });
      }
      if (this.#f) {
        this.#t.before(this.#f);
        this.#f = null;
      }
    }
  }
  update_pending_count(t, n) {
    this.#k(t, n);
    this.#h += t;
    if (!!this.#a && !this.#v) {
      this.#v = true;
      ae(() => {
        this.#v = false;
        if (this.#a) {
          Fe(this.#a, this.#h);
        }
      });
    }
  }
  get_effect_pending() {
    this.#_();
    return g(this.#a);
  }
  error(t) {
    if (t === cn || !this.#e.onerror && !this.#e.failed) {
      throw t;
    }
    if (m?.is_fork) {
      if (this.#r) {
        m.skip_effect(this.#r);
      }
      if (this.#s) {
        m.skip_effect(this.#s);
      }
      if (this.#i) {
        m.skip_effect(this.#i);
      }
      m.oncommit(() => {
        if (!this.#c()) {
          this.#x(t);
        }
      });
    } else {
      this.#x(t);
    }
  }
  #x(t) {
    if (this.#r) {
      L(this.#r);
      this.#r = null;
    }
    if (this.#s) {
      L(this.#s);
      this.#s = null;
    }
    if (this.#i) {
      L(this.#i);
      this.#i = null;
    }
    const n = this.#e.failed;
    const r = (i) => {
      if (this.#c()) {
        return;
      }
      const { reset: s, invoke_onerror: l } = this.#m(i);
      l();
      if (n && !this.#c()) {
        this.#i = this.#y(() => {
          try {
            return V(() => {
              const f = y;
              f.b = this;
              f.f |= Et;
              n(this.#t, () => i, () => s);
            });
          } catch (f) {
            Q(f, this.#n.parent);
            return null;
          }
        });
      }
    };
    ae(() => {
      if (!this.#c()) {
        var i;
        try {
          i = this.transform_error(t);
        } catch (s) {
          Q(s, this.#n && this.#n.parent);
          return;
        }
        if (i !== null && typeof i == "object" && typeof i.then == "function") {
          i.then(r, (s) => Q(s, this.#n && this.#n.parent));
        } else {
          r(i);
        }
      }
    });
  }
};
function J(e, t) {
  const n = t == null ? "" : typeof t == "object" ? `${t}` : t;
  if (n !== (e[xt] ??= e.nodeValue)) {
    e[xt] = n;
    e.nodeValue = `${n}`;
  }
}
function si(e, t) {
  return li(e, t);
}
const tt = new Map();
function li(e, { target: t, anchor: n, props: r = {}, events: i, context: s, intro: l = true, transformError: f }) {
  Ir();
  let o = undefined;
  const u = qr(() => {
    const a = n ?? t.appendChild(he());
    Yn(a, { pending: () => {} }, (h) => {
      vt({});
      const v = Z;
      if (s) {
        v.c = s;
      }
      if (i) {
        r.$$events = i;
      }
      o = e(h, r) || vn();
      _t();
    }, f);
    const c = new Set();
    const d = (h) => {
      for (let v = 0; v < h.length; v++) {
        const b = h[v];
        if (!c.has(b)) {
          c.add(b);
          const _ = Zr(b);
          for (const A of [t, document]) {
            let p = tt.get(A);
            if (p === undefined) {
              p = new Map();
              tt.set(A, p);
            }
            const E = p.get(b);
            if (E === undefined) {
              A.addEventListener(b, tn, { passive: _ });
              p.set(b, 1);
            } else {
              p.set(b, E + 1);
            }
          }
        }
      }
    };
    d(dt(zn));
    Ct.add(d);
    return () => {
      for (const h of c) {
        for (const _ of [t, document]) {
          const v = tt.get(_);
          let b = v.get(h);
          if (--b == 0) {
            _.removeEventListener(h, tn);
            v.delete(h);
            if (v.size === 0) {
              tt.delete(_);
            }
          } else {
            v.set(h, b);
          }
        }
      }
      Ct.delete(d);
      if (a !== n) {
        a.parentNode?.removeChild(a);
      }
    };
  });
  fi.set(o, u);
  return o;
}
let fi = new WeakMap();
class oi {
  anchor;
  #t = new Map();
  #o = new Map();
  #e = new Map();
  #l = new Set();
  #n = true;
  constructor(t, n = true) {
    this.anchor = t;
    this.#n = n;
  }
  #r = (t) => {
    if (this.#t.has(t)) {
      const n = this.#t.get(t);
      let r = this.#o.get(n);
      if (r) {
        ct(r);
        this.#l.delete(n);
      } else {
        const i = this.#e.get(n);
        if (i) {
          ct(i.effect);
          this.#o.set(n, i.effect);
          this.#e.delete(n);
          i.fragment.lastChild.remove();
          this.anchor.before(i.fragment);
          r = i.effect;
        }
      }
      for (const [s, l] of this.#t) {
        this.#t.delete(s);
        if (s === t) {
          break;
        }
        const f = this.#e.get(l);
        if (f) {
          L(f.effect);
          this.#e.delete(l);
        }
      }
      for (const [s, l] of this.#o) {
        if (s === n || this.#l.has(s)) {
          continue;
        }
        const f = () => {
          if (Array.from(this.#t.values()).includes(s)) {
            const u = document.createDocumentFragment();
            zt(l, u);
            u.append(he());
            this.#e.set(s, {
              effect: l,
              fragment: u
            });
          } else {
            L(l);
          }
          this.#l.delete(s);
          this.#o.delete(s);
        };
        if (this.#n || !r) {
          this.#l.add(s);
          ke(l, f, false);
        } else {
          f();
        }
      }
    }
  };
  #s = (t) => {
    this.#t.delete(t);
    const n = Array.from(this.#t.values());
    for (const [r, i] of this.#e) {
      if (!n.includes(r)) {
        L(i.effect);
        this.#e.delete(r);
      }
    }
  };
  ensure(t, n) {
    const r = m;
    const i = Mn();
    if (n && !this.#o.has(t) && !this.#e.has(t)) {
      if (i) {
        const s = document.createDocumentFragment();
        var l = he();
        s.append(l);
        this.#e.set(t, {
          effect: V(() => n(l)),
          fragment: s
        });
      } else {
        this.#o.set(t, V(() => n(this.anchor)));
      }
    }
    this.#t.set(r, t);
    if (i) {
      for (const [f, o] of this.#o) {
        if (f === t) {
          r.unskip_effect(o);
        } else {
          r.skip_effect(o);
        }
      }
      for (const [f, o] of this.#e) {
        if (f === t) {
          r.unskip_effect(o.effect);
        } else {
          r.skip_effect(o.effect);
        }
      }
      r.oncommit(this.#r);
      r.ondiscard(this.#s);
    } else {
      this.#r(r);
    }
  }
}
function ui(e, t) {
  return t;
}
function ai(e, t, n) {
  const r = [];
  for (var i = t.length, s, l = t.length, f = 0; f < i; f++) {
    const c = t[f];
    ke(c, () => {
      if (s) {
        s.pending.delete(c);
        s.done.add(c);
        if (s.pending.size === 0) {
          const d = e.outrogroups;
          Ot(e, dt(s.done));
          d.delete(s);
          if (d.size === 0) {
            e.outrogroups = null;
          }
        }
      } else {
        l -= 1;
      }
    }, false);
  }
  if (l === 0) {
    const o = r.length === 0 && n !== null && e.pending.size === 0;
    if (o) {
      const u = n;
      const a = u.parentNode;
      Lr(a);
      a.append(u);
      e.items.clear();
    }
    Ot(e, t, !o);
  } else {
    s = {
      pending: new Set(t),
      done: new Set()
    };
    (e.outrogroups ??= new Set()).add(s);
  }
}
function Ot(e, t, n = true) {
  let r;
  if (e.pending.size > 0) {
    r = new Set();
    for (const l of e.pending.values()) {
      for (const f of l) {
        r.add(e.items.get(f).e);
      }
    }
  }
  for (let i = 0; i < t.length; i++) {
    const s = t[i];
    if (r?.has(s)) {
      s.f |= ee;
      const l = document.createDocumentFragment();
      zt(s, l);
    } else {
      L(t[i], n);
    }
  }
}
var nn;
function Mt(e, t, n, r, i, s = null) {
  var l = e;
  const f = new Map();
  const o = (t & an) !== 0;
  if (o) {
    const u = e;
    l = u.appendChild(he());
  }
  var a = null;
  var c = St(() => {
    const E = n();
    if (sn(E)) {
      return E;
    } else if (E == null) {
      return [];
    } else {
      return dt(E);
    }
  });
  const d = new Map();
  let h = true;
  function v(E) {
    if ((p.effect.f & O) === 0) {
      p.pending.delete(E);
      const A = g(c);
      p.fallback = a;
      ci(p, A, l, t, r);
      if (a !== null) {
        if (A.length === 0) {
          if ((a.f & ee) === 0) {
            ct(a);
          } else {
            a.f ^= ee;
            ze(a, null, l);
          }
        } else {
          ke(a, () => {
            a = null;
          });
        }
      }
    }
  }
  function b(E) {
    p.pending.delete(E);
  }
  const _ = qt(() => {
    const E = g(c);
    for (var A = E.length, C = new Set(), U = m, oe = Mn(), $ = 0; $ < A; $ += 1) {
      const me = E[$];
      const Ce = r(me, $);
      let R = h ? null : f.get(Ce);
      if (R) {
        if (R.v) {
          Fe(R.v, me);
        }
        if (R.i) {
          Fe(R.i, $);
        }
        if (oe) {
          U.unskip_effect(R.e);
        }
      } else {
        R = di(f, h ? l : nn ??= he(), me, Ce, $, i, t, n);
        if (!h) {
          R.e.f |= ee;
        }
        f.set(Ce, R);
      }
      C.add(Ce);
    }
    if (A === 0 && s && !a) {
      if (h) {
        a = V(() => s(l));
      } else {
        a = V(() => s(nn ??= he()));
        a.f |= ee;
      }
    }
    if (A > C.size) {
      pr();
    }
    if (!h) {
      d.set(U, C);
      if (oe) {
        for (const [Gn, Kn] of f) {
          if (!C.has(Gn)) {
            U.skip_effect(Kn.e);
          }
        }
        U.oncommit(v);
        U.ondiscard(b);
      } else {
        v(U);
      }
    }
    g(c);
  });
  var p = {
    effect: _,
    items: f,
    pending: d,
    outrogroups: null,
    fallback: a
  };
  h = false;
}
function He(e) {
  while (e !== null && (e.f & re) === 0) {
    e = e.next;
  }
  return e;
}
function ci(e, t, n, r, i) {
  const s = (r & or) !== 0;
  const l = t.length;
  const f = e.items;
  let o = He(e.effect.first);
  let a = null;
  let d = [];
  let h = [];
  let c;
  let v;
  let b;
  let _;
  let p;
  if (s) {
    for (p = 0; p < l; p += 1) {
      v = t[p];
      b = i(v, p);
      _ = f.get(b).e;
      if ((_.f & ee) === 0) {
        _.nodes?.a?.measure();
        (c ??= new Set()).add(_);
      }
    }
  }
  let u;
  for (p = 0; p < l; p += 1) {
    v = t[p];
    b = i(v, p);
    _ = f.get(b).e;
    if (e.outrogroups !== null) {
      for (const R of e.outrogroups) {
        R.pending.delete(_);
        R.done.delete(_);
      }
    }
    if ((_.f & F) !== 0) {
      ct(_);
      if (s) {
        _.nodes?.a?.unfix();
        (c ??= new Set()).delete(_);
      }
    }
    if ((_.f & ee) !== 0) {
      _.f ^= ee;
      if (_ === o) {
        ze(_, null, n);
      } else {
        const E = a ? a.next : o;
        if (_ === e.effect.last) {
          e.effect.last = _.prev;
        }
        if (_.prev) {
          _.prev.next = _.next;
        }
        if (_.next) {
          _.next.prev = _.prev;
        }
        ue(e, a, _);
        ue(e, _, E);
        ze(_, E, n);
        a = _;
        d = [];
        h = [];
        o = He(a.next);
        continue;
      }
    }
    if (_ !== o) {
      if (u !== undefined && u.has(_)) {
        if (d.length < h.length) {
          const A = h[0];
          var C;
          a = A.prev;
          const U = d[0];
          const oe = d[d.length - 1];
          for (C = 0; C < d.length; C += 1) {
            ze(d[C], A, n);
          }
          for (C = 0; C < h.length; C += 1) {
            u.delete(h[C]);
          }
          ue(e, U.prev, oe.next);
          ue(e, a, U);
          ue(e, oe, A);
          o = A;
          a = oe;
          p -= 1;
          d = [];
          h = [];
        } else {
          u.delete(_);
          ze(_, o, n);
          ue(e, _.prev, _.next);
          ue(e, _, a === null ? e.effect.first : a.next);
          ue(e, a, _);
          a = _;
        }
        continue;
      }
      d = [];
      h = [];
      while (o !== null && o !== _) {
        (u ??= new Set()).add(o);
        h.push(o);
        o = He(o.next);
      }
      if (o === null) {
        continue;
      }
    }
    if ((_.f & ee) === 0) {
      d.push(_);
    }
    a = _;
    o = He(_.next);
  }
  if (e.outrogroups !== null) {
    for (const R of e.outrogroups) {
      if (R.pending.size === 0) {
        Ot(e, dt(R.done));
        e.outrogroups?.delete(R);
      }
    }
    if (e.outrogroups.size === 0) {
      e.outrogroups = null;
    }
  }
  if (o !== null || u !== undefined) {
    const $ = [];
    if (u !== undefined) {
      for (_ of u) {
        if ((_.f & F) === 0) {
          $.push(_);
        }
      }
    }
    while (o !== null) {
      if ((o.f & F) === 0 && o !== e.fallback) {
        $.push(o);
      }
      o = He(o.next);
    }
    const me = $.length;
    if (me > 0) {
      const Ce = (r & an) !== 0 && l === 0 ? n : null;
      if (s) {
        for (p = 0; p < me; p += 1) {
          $[p].nodes?.a?.measure();
        }
        for (p = 0; p < me; p += 1) {
          $[p].nodes?.a?.fix();
        }
      }
      ai(e, $, Ce);
    }
  }
  if (s) {
    ae(() => {
      if (c !== undefined) {
        for (_ of c) {
          _.nodes?.a?.apply();
        }
      }
    });
  }
}
function di(e, t, n, r, i, s, l, f) {
  const o = (l & lr) !== 0 ? (l & ur) === 0 ? Dr(n, false, false) : Se(n) : null;
  const u = (l & fr) !== 0 ? Se(i) : null;
  return {
    v: o,
    i: u,
    e: V(() => {
      s(t, o ?? n, u ?? i, f);
      return () => {
        e.delete(r);
      };
    })
  };
}
function ze(e, t, n) {
  if (e.nodes) {
    for (let r = e.nodes.start, i = e.nodes.end, s = t && (t.f & ee) === 0 ? t.nodes.start : n; r !== null;) {
      const l = Qe(r);
      s.before(r);
      if (r === i) {
        return;
      }
      r = l;
    }
  }
}
function ue(e, t, n) {
  if (t === null) {
    e.effect.first = n;
  } else {
    t.next = n;
  }
  if (n === null) {
    e.effect.last = t;
  } else {
    n.prev = t;
  }
}
function hi(e, t, ...n) {
  const r = new oi(e);
  qt(() => {
    const i = t() ?? null;
    r.ensure(i, i && ((s) => i(s, ...n)));
  }, De);
}
function vi(e, t) {
  Vr(() => {
    e = y?.parent?.nodes?.start ?? e;
    const n = e.getRootNode();
    const r = n.host ? n : n.head ?? n.ownerDocument.head;
    if (!r.querySelector("#" + t.hash)) {
      const i = Nn("style");
      i.id = t.hash;
      i.textContent = t.code;
      r.appendChild(i);
    }
  });
}
const rn = [...` 	
\r\f \v\uFEFF`];
function _i(e, t, n) {
  let r = e == null ? "" : "" + e;
  if (t) {
    r = r ? r + " " + t : t;
  }
  if (n) {
    for (const i of Object.keys(n)) {
      if (n[i]) {
        r = r ? r + " " + i : i;
      } else if (r.length) {
        const s = i.length;
        for (let l = 0; (l = r.indexOf(i, l)) >= 0;) {
          const f = l + s;
          if ((l === 0 || rn.includes(r[l - 1])) && (f === r.length || rn.includes(r[f]))) {
            r = (l === 0 ? "" : r.substring(0, l)) + r.substring(f + 1);
          } else {
            l = f;
          }
        }
      }
    }
  }
  if (r === "") {
    return null;
  } else {
    return r;
  }
}
function Un(e, t, n, r, i, s) {
  const l = e[kt];
  if (l !== n || l === undefined) {
    const f = _i(n, r, s);
    if (f == null) {
      e.removeAttribute("class");
    } else {
      e.className = f;
    }
    e[kt] = n;
  } else if (s && i !== s) {
    for (const o in s) {
      const u = !!s[o];
      if (i == null || u !== !!i[o]) {
        e.classList.toggle(o, u);
      }
    }
  }
  return s;
}
function pi(e, t, n = t) {
  const r = new WeakSet();
  mn(e, "input", async (i) => {
    let s = i ? e.defaultValue : e.value;
    s = bt(e) ? yt(s) : s;
    n(s);
    if (m !== null) {
      r.add(m);
    }
    await Wr();
    if (s !== (s = t())) {
      const { selectionStart: l, selectionEnd: f } = e;
      const o = e.value.length;
      e.value = s ?? "";
      if (f !== null) {
        const u = e.value.length;
        if (l === f && f === o && u > o) {
          e.selectionStart = u;
          e.selectionEnd = u;
        } else {
          e.selectionStart = l;
          e.selectionEnd = Math.min(f, u);
        }
      }
    }
  });
  if (Yt(t) == null && e.value) {
    n(bt(e) ? yt(e.value) : e.value);
    if (m !== null) {
      r.add(m);
    }
  }
  Ht(() => {
    const i = t();
    if (e === document.activeElement) {
      const s = m;
      if (r.has(s)) {
        return;
      }
    }
    if ((!bt(e) || i !== yt(e.value)) && (e.type !== "date" || !!i || !!e.value)) {
      if (i !== e.value) {
        e.value = i ?? "";
      }
    }
  });
}
function gi(e, t, n = t) {
  mn(e, "change", (r) => {
    const i = r ? e.defaultChecked : e.checked;
    n(i);
  });
  if (Yt(t) == null) {
    n(e.checked);
  }
  Ht(() => {
    const r = t();
    e.checked = !!r;
  });
}
function bt(e) {
  const t = e.type;
  return t === "number" || t === "range";
}
function yt(e) {
  if (e === "") {
    return null;
  } else {
    return +e;
  }
}
const mi = {
  get(e, t) {
    let n = e.props.length;
    while (n--) {
      let r = e.props[n];
      if (Be(r)) {
        r = r();
      }
      if (typeof r == "object" && r !== null && t in r) {
        return r[t];
      }
    }
  },
  set(e, t, n) {
    let r = e.props.length;
    while (r--) {
      let i = e.props[r];
      if (Be(i)) {
        i = i();
      }
      const s = Ee(i, t);
      if (s && s.set) {
        s.set(n);
        return true;
      }
    }
    return false;
  },
  getOwnPropertyDescriptor(e, t) {
    let n = e.props.length;
    while (n--) {
      let r = e.props[n];
      if (Be(r)) {
        r = r();
      }
      if (typeof r == "object" && r !== null && t in r) {
        const i = Ee(r, t);
        if (i && !i.configurable) {
          i.configurable = true;
        }
        return i;
      }
    }
  },
  has(e, t) {
    if (t === nt || t === rr) {
      return false;
    }
    for (let n of e.props) {
      if (Be(n)) {
        n = n();
      }
      if (n != null && t in n) {
        return true;
      }
    }
    return false;
  },
  ownKeys(e) {
    const t = [];
    for (let n of e.props) {
      if (Be(n)) {
        n = n();
      }
      if (n) {
        for (const r in n) {
          if (!t.includes(r)) {
            t.push(r);
          }
        }
        for (const r of Object.getOwnPropertySymbols(n)) {
          if (!t.includes(r)) {
            t.push(r);
          }
        }
      }
    }
    return t;
  }
};
function wi(...e) {
  return new Proxy({ props: e }, mi);
}
function bi(e, t, n, r) {
  let i = r;
  let s = true;
  const l = () => {
    if (s) {
      s = false;
      i = r;
    }
    return i;
  };
  let f;
  f = e[t];
  if (f === undefined && r !== undefined) {
    f = l();
  }
  let o;
  o = () => {
    const u = e[t];
    if (u === undefined) {
      return l();
    } else {
      s = true;
      return u;
    }
  };
  return o;
}
const yi = "5";
if (typeof window !== "undefined") {
  ((window.__svelte ??= {}).v ??= new Set()).add(yi);
}
var Ei = Y("<span> <!> <button>pick</button></span>");
function Nt(e, t) {
  vt(t, true);
  const n = bi(t, "tone", 3, "info");
  const r = Ei();
  const i = On(r);
  const s = P(i);
  hi(s, () => t.children ?? st);
  const l = P(s, 2);
  ie(() => {
    Un(r, 1, `badge ${n() ?? ""}`);
    J(i, `${t.label ?? ""} `);
  });
  ge("click", l, () => t.onpick?.(t.label));
  H(e, r);
  _t();
}
Ae(["click"]);
var ki = Y("<!> <!> <p> </p> <button class=\"more\">more</button>", 1);
function xi(e) {
  const t = N("none");
  const n = N(1);
  const r = ki();
  const i = je(r);
  Nt(i, {
    label: "first",
    onpick: (u) => D(t, u, true),
    children: (u, a) => {
      const c = ti();
      ie(() => J(c, `inner ${g(n) ?? ""}`));
      H(u, c);
    },
    $$slots: { default: true }
  });
  const s = P(i, 2);
  Nt(s, {
    label: "second",
    tone: "warn"
  });
  const l = P(s, 2);
  const f = pe(l);
  const o = P(l, 2);
  ie(() => J(f, `picked ${g(t) ?? ""}`));
  ge("click", o, () => Fr(n));
  H(e, r);
}
Ae(["click"]);
var Si = Y("<button> </button> <p> </p>", 1);
function Ti(e) {
  const t = N(0);
  const n = It(() => g(t) * 2);
  function r() {
    D(t, g(t) + 1);
  }
  const i = Si();
  const s = je(i);
  const l = pe(s);
  const f = P(s, 2);
  const o = pe(f);
  ie(() => {
    J(l, `clicks: ${g(t) ?? ""}`);
    J(o, `doubled is ${g(n) ?? ""}`);
  });
  ge("click", s, r);
  H(e, i);
}
Ae(["click"]);
var Ai = Y("<p> </p>");
var Ci = Y("<span> </span>");
var Ri = Y("<!> <!> <button>flip</button>", 1);
function Oi(e) {
  const t = ce([{
    id: 1,
    pos: { x: 3 }
  }, {
    id: 2,
    pos: {}
  }]);
  const n = Ri();
  const r = je(n);
  Mt(r, 19, () => t, ({ id: l, pos: { x: f = 9 } }) => l, (l, f, o) => {
    const u = () => g(f).id;
    const a = St(() => Gt(g(f).pos.x, 9));
    const c = Ai();
    const d = pe(c);
    ie(() => J(d, `${g(o) ?? ""}: ${u() ?? ""} at ${g(a) ?? ""}`));
    H(l, c);
  });
  const i = P(r, 2);
  Mt(i, 16, () => [[1, 2], [3]], ui, (l, f) => {
    const o = It(() => er(f, 2));
    const u = () => g(o)[0];
    const a = St(() => Gt(g(o)[1], 0));
    const c = Ci();
    const d = pe(c);
    ie(() => J(d, `${u() ?? ""}-${g(a) ?? ""}`));
    H(l, c);
  });
  const s = P(i, 2);
  ge("click", s, () => t.reverse());
  H(e, n);
}
Ae(["click"]);
var Mi = Y("<p class=\"err\"> </p>");
var Ni = Y("<p> </p>");
var Di = Y("<!> <button>break</button>", 1);
function Pi(e, t) {
  vt(t, true);
  const n = N(false);
  function r() {
    if (g(n)) {
      throw new Error("boom");
    }
    return "fine";
  }
  const i = Di();
  const s = je(i);
  Yn(s, { failed: (o, u = st, a = st) => {
    const c = Mi();
    const d = pe(c, true);
    ie(() => J(d, u().message));
    H(o, c);
  } }, (o) => {
    const u = Ni();
    const a = pe(u, true);
    ie((c) => J(a, c), [() => r()]);
    H(o, u);
  });
  const l = P(s, 2);
  ge("click", l, () => D(n, true));
  H(e, i);
  _t();
}
Ae(["click"]);
var Fi = Y("<!> <button class=\"swap\">swap</button>", 1);
function Ii(e) {
  const t = N(ce({
    label: "spread",
    tone: "warn"
  }));
  const n = Fi();
  const r = je(n);
  Nt(r, wi(() => g(t)));
  const i = P(r, 2);
  ge("click", i, () => D(t, { label: "other" }, true));
  H(e, n);
}
Ae(["click"]);
var Li = Y("<li><input type=\"checkbox\"/> <button class=\"remove\">x</button></li>");
var $i = Y("<input placeholder=\"new todo\"/> <button class=\"add\">add</button> <ul></ul> <p> </p>", 1);
const ji = {
  hash: "svelte-70rljk",
  code: ".done.svelte-70rljk {text-decoration:line-through;}"
};
function Bi(e, t) {
  vt(t, true);
  vi(e, ji);
  const n = N(ce([{
    id: 1,
    text: "write",
    done: false
  }]));
  const r = N("");
  let i = 2;
  const s = It(() => g(n).filter((v) => !v.done).length);
  function l() {
    if (g(r)) {
      g(n).push({
        id: i++,
        text: g(r),
        done: false
      });
      D(r, "");
    }
  }
  function f(v) {
    D(n, g(n).filter((b) => b.id !== v), true);
  }
  const o = $i();
  const u = je(o);
  const a = P(u, 2);
  const c = P(a, 2);
  Mt(c, 21, () => g(n), (v) => v.id, (v, b, _) => {
    const p = Li();
    let E;
    const A = On(p);
    const C = P(A);
    const U = P(C);
    ie(() => {
      E = Un(p, 1, "svelte-70rljk", null, E, { done: g(b).done });
      J(C, ` ${g(b).text ?? ""} `);
    });
    gi(A, () => g(b).done, (oe) => g(b).done = oe);
    ge("click", U, () => f(g(b).id));
    H(v, p);
  });
  const d = P(c, 2);
  const h = pe(d);
  ie(() => J(h, `${g(s) ?? ""} left`));
  pi(u, () => g(r), (v) => D(r, v));
  ge("click", a, l);
  H(e, o);
  _t();
}
Ae(["click"]);
window.__mount = si;
window.__component = [xi, Ti, Oi, Pi, Ii, Bi];