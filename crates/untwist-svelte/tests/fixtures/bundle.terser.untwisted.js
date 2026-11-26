(function () {
  "use strict";
  var e = false;
  var t = Array.isArray;
  var n = Array.prototype.indexOf;
  var r = Array.prototype.includes;
  var i = Array.from;
  var s = Object.defineProperty;
  var l = Object.getOwnPropertyDescriptor;
  var o = Object.prototype;
  var f = Array.prototype;
  var u = Object.getPrototypeOf;
  const a = Object.isExtensible;
  function c(e) {
    return typeof e == "function";
  }
  const d = () => {};
  function h() {
    let e;
    let t;
    return {
      promise: new Promise((n, r) => {
        e = n;
        t = r;
      }),
      resolve: e,
      reject: t
    };
  }
  function v(e, t, n = false) {
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
  const p = 16;
  const _ = 32;
  const g = 64;
  const m = 512;
  const y = 1024;
  const b = 2048;
  const w = 4096;
  const k = 8192;
  const x = 16384;
  const S = 32768;
  const j = 33554432;
  const M = 65536;
  const P = 131072;
  const E = 524288;
  const $ = 33554432;
  const O = 2097152;
  const q = 8388608;
  const A = Symbol("$state");
  const T = Symbol("component");
  const z = Symbol("legacy props");
  const C = Symbol("attributes");
  const D = Symbol("class");
  const N = Symbol("style");
  const L = Symbol("text");
  const R = Symbol("form reset");
  const F = new class extends Error {
    name = "StaleReactionError";
    message = "The reaction that called `getAbortSignal()` was re-run or destroyed";
  }();
  const H = {};
  const K = Symbol("uninitialized");
  function V(e) {
    return e === this.v;
  }
  function W(e) {
    t = e;
    n = this.v;
    return !(t != t ? n == n : t !== n || t !== null && typeof t == "object" || typeof t == "function");
    var t;
    var n;
  }
  let B = null;
  function I(e) {
    B = e;
  }
  function G(e, t = false, n) {
    B = {
      p: B,
      i: false,
      c: null,
      e: null,
      s: e,
      x: null,
      r: Ot,
      l: null
    };
  }
  function J(e) {
    const t = B;
    const n = t.e;
    if (n !== null) {
      t.e = null;
      for (const r of n) {
        ct(r);
      }
    }
    t.i = true;
    B = t.p;
    return Q(e);
  }
  function Q(e = {}) {
    s(e, T, { value: true });
    return e;
  }
  let U = [];
  function X() {
    const e = U;
    U = [];
    (function (e) {
      for (let t = 0; t < e.length; t++) {
        e[t]();
      }
    })(e);
  }
  function Y(e) {
    if (U.length === 0 && !xe) {
      var t = U;
      queueMicrotask(() => {
        if (t === U) {
          X();
        }
      });
    }
    U.push(e);
  }
  function Z() {
    while (U.length > 0) {
      X();
    }
  }
  const ee = -7169;
  function te(e, t) {
    e.f = e.f & ee | t;
  }
  function ne(e) {
    if ((e.f & m) !== 0 || e.deps === null) {
      te(e, y);
    } else {
      te(e, w);
    }
  }
  function re(e, t) {
    if ((e.f & b) !== 0) {
      t.set(e, b);
    } else if ((e.f & w) !== 0 && t.get(e) !== b) {
      t.set(e, w);
    }
    te(e, y);
  }
  let ie = false;
  function se(e) {
    const t = Pt;
    const n = Ot;
    $t(null);
    qt(null);
    try {
      return e();
    } finally {
      $t(t);
      qt(n);
    }
  }
  function le(e, t, n, r = n) {
    e.addEventListener(t, () => se(n));
    const i = e[R];
    e[R] = i ? () => {
      i();
      r(true);
    } : () => r(true);
    if (!ie) {
      ie = true;
      document.addEventListener("reset", (e) => {
        Promise.resolve().then(() => {
          if (!e.defaultPrevented) {
            for (const t of e.target.elements) {
              t[R]?.();
            }
          }
        });
      }, { capture: true });
    }
  }
  function oe(e, t, n, r) {
    const i = ae;
    const s = e.filter((e) => !e.settled);
    var l = t.map(i);
    if (n.length !== 0 || s.length !== 0) {
      var o = Ot;
      var f = function () {
        var e = Ot;
        var t = Pt;
        var n = B;
        var r = ye;
        return function (i = true) {
          qt(e);
          $t(t);
          I(n);
          if (i && (e.f & x) === 0) {
            r?.activate();
            r?.apply();
          }
        };
      }();
      const u = s.length === 1 ? s[0].promise : s.length > 1 ? Promise.all(s.map((e) => e.promise)) : null;
      var a = ue();
      if (n.length !== 0) {
        if (u) {
          u.then(() => {
            if ((o.f & x) === 0) {
              f();
              d();
              fe();
            } else {
              a();
            }
          });
        } else {
          d();
        }
      } else {
        u.then(() => c([])).finally(a);
      }
    } else {
      r(l);
    }
    function c(e) {
      if ((o.f & x) === 0) {
        f();
        try {
          r([...l, ...e]);
        } catch (e) {
          ft(e, o);
        }
        fe();
      }
    }
    function d() {
      Promise.all(n.map((e) => function (e) {
        const t = Ot;
        if (t === null) {
          (function () {
            throw new Error("https://svelte.dev/e/async_derived_orphan");
          })();
        }
        let n = undefined;
        const r = He(K);
        const i = !Pt;
        const s = new Set();
        (function (e) {
          ut(4718592, e);
        })(() => {
          const l = Ot;
          const o = h();
          n = o.promise;
          try {
            Promise.resolve(e()).then(o.resolve, (e) => {
              if (e !== F) {
                o.reject(e);
              }
            }).finally(fe);
          } catch (e) {
            o.reject(e);
            fe();
          }
          const f = ye;
          if (i) {
            if ((l.f & S) !== 0) {
              var u = ue();
            }
            if (t.b?.is_rendered()) {
              f.async_deriveds.get(l)?.reject(ce);
            } else {
              for (const e of s.values()) {
                e.reject(ce);
              }
            }
            s.add(o);
            f.async_deriveds.set(l, o);
          }
          const a = (e, t = undefined) => {
            u?.();
            s.delete(o);
            if (t !== ce) {
              f.activate();
              if (t) {
                r.f |= q;
                Ue(r, t);
              } else {
                if ((r.f & q) !== 0) {
                  r.f ^= q;
                }
                Ue(r, e);
              }
              f.deactivate();
            }
          };
          o.promise.then(a, (e) => a(null, e || "unknown"));
        });
        (function (e) {
          const t = ut(8, null);
          te(t, y);
          t.teardown = e;
        })(() => {
          for (const e of s) {
            e.reject(ce);
          }
        });
        return new Promise((e) => {
          function t(i) {
            function s() {
              if (i === n) {
                e(r);
              } else {
                t(n);
              }
            }
            i.then(s, s);
          }
          t(n);
        });
      }(e))).then(c).catch((e) => ft(e, o)).finally(a);
    }
  }
  function fe(e = true) {
    qt(null);
    $t(null);
    I(null);
    if (e) {
      ye?.deactivate();
    }
  }
  function ue() {
    const e = Ot;
    const t = e.b;
    const n = ye;
    const r = !!t?.is_rendered();
    t?.update_pending_count(1, n);
    n.increment(r, e);
    return () => {
      t?.update_pending_count(-1, n);
      n.decrement(r, e);
    };
  }
  function ae(e) {
    if (Ot !== null) {
      Ot.f |= E;
    }
    return {
      ctx: B,
      deps: null,
      effects: null,
      equals: V,
      f: 2050,
      fn: e,
      reactions: null,
      rv: 0,
      v: K,
      wv: 0,
      parent: Ot,
      ac: null
    };
  }
  const ce = Symbol("obsolete");
  function de(e) {
    const t = ae(e);
    Tt(t);
    return t;
  }
  function he(e) {
    const t = ae(e);
    t.equals = W;
    return t;
  }
  function ve(e) {
    const n = Ot;
    const r = e.parent;
    if (!jt && r !== null && e.v !== K && r.f & 24576) {
      console.warn("https://svelte.dev/e/derived_inert");
      return e.v;
    }
    qt(r);
    let t;
    try {
      (function (e) {
        const t = e.effects;
        if (t !== null) {
          e.effects = null;
          for (let n = 0; n < t.length; n += 1) {
            mt(t[n]);
          }
        }
      })(e);
      t = Wt(e);
    } finally {
      qt(n);
    }
    return t;
  }
  function pe(e) {
    const t = ve(e);
    if (e.equals(t) || (e.wv = Ht(), ye?.is_fork && e.deps !== null || (ye !== null ? (ye.capture(e, t, true), be?.capture(e, t, true)) : e.v = t, e.deps !== null))) {
      if (!jt) {
        if (we !== null) {
          if (at() || ye?.is_fork) {
            we.set(e, t);
          }
        } else {
          ne(e);
        }
      }
    } else {
      te(e, y);
    }
  }
  function _e(e) {
    if (e.effects !== null) {
      for (const t of e.effects) {
        if (t.teardown && t.fn !== null) {
          Jt(t);
        }
      }
    }
  }
  let ge = null;
  let me = null;
  let ye = null;
  let be = null;
  let we = null;
  let ke = null;
  let xe = false;
  let Se = false;
  let je = null;
  let Me = null;
  var Pe = 0;
  new Set();
  let Ee = 1;
  class $e {
    id = Ee++;
    started = false;
    linked = true;
    prev = null;
    next = null;
    async_deriveds = new Map();
    current = new Map();
    previous = new Map();
    #e = new Set();
    #t = new Set();
    #n = 0;
    #r = new Map();
    #i = null;
    #s = [];
    #l = [];
    #o = new Map();
    #f = new Map();
    #u = new Set();
    is_fork = false;
    #a = false;
    constructor() {
      if (me === null) {
        ge = me = this;
      } else {
        me.next = this;
        this.prev = me;
      }
      me = this;
    }
    #c() {
      if (this.is_fork) {
        return true;
      }
      for (const e of this.#r.keys()) {
        if (!this.#d(e)) {
          return true;
        }
      }
      return false;
    }
    #d(e) {
      for (let t = e; t !== null; t = t.parent) {
        if (this.#f.has(t)) {
          return true;
        }
      }
      return false;
    }
    skip_effect(e) {
      if (!this.#f.has(e)) {
        this.#f.set(e, {
          d: [],
          m: []
        });
      }
      this.#u.delete(e);
    }
    unskip_effect(e, t = (e) => this.schedule(e)) {
      const n = this.#f.get(e);
      if (n) {
        this.#f.delete(e);
        for (var r of n.d) {
          te(r, b);
          t(r);
        }
        for (r of n.m) {
          te(r, w);
          t(r);
        }
      }
      this.#u.add(e);
    }
    #h() {
      const e = [];
      for (const i of this.#s) {
        if ((i.f & x) === 0 && i.f & 6144) {
          for (var t = i, n = false; t.parent !== null;) {
            const r = (t = t.parent).f;
            if (r & 96) {
              if ((r & y) === 0) {
                n = true;
                break;
              }
              t.f ^= y;
            }
          }
          if (!n) {
            e.push(t);
          }
        }
      }
      this.#s = [];
      return e;
    }
    #v() {
      this.started = true;
      for (const [e, t] of this.#o) {
        te(e, t);
        this.schedule(e);
      }
      this.apply();
      var e = je = [];
      var t = [];
      const n = Me = [];
      while (this.#s.length > 0) {
        if (Pe++ > 1e3) {
          this.#p();
          Oe();
        }
        for (const n of this.#h()) {
          try {
            this.#_(n, e, t);
          } catch (e) {
            Ne(n);
            if (!this.#c()) {
              this.discard();
            }
            throw e;
          }
        }
      }
      ye = null;
      if (n.length > 0) {
        const r = $e.ensure();
        for (const e of n) {
          r.schedule(e);
        }
      }
      je = null;
      Me = null;
      if (this.#c()) {
        this.#g(t);
        this.#g(e);
        for (const [e, t] of this.#f) {
          De(e, t);
        }
        if (n.length > 0) {
          ye.#v();
        }
        return;
      }
      const i = this.#m();
      if (i) {
        this.#g(t);
        this.#g(e);
        i.#y(this);
        return;
      }
      this.#o.clear();
      for (const e of this.#e) {
        e(this);
      }
      this.#e.clear();
      be = this;
      Ae(t);
      Ae(e);
      be = null;
      this.#i?.resolve();
      const s = ye;
      if (this.#n === 0 && (this.#s.length === 0 || s !== null)) {
        this.#p();
      }
      if (s !== null) {
        Re.clear();
        s.#v();
      }
    }
    #_(e, t, n) {
      e.f ^= y;
      for (let r = e.first; r !== null;) {
        const i = r.f;
        const s = !!(i & 96);
        if ((!s || (i & y) === 0) && (i & k) === 0 && !this.#f.has(r) && r.fn !== null) {
          if (s) {
            r.f ^= y;
          } else if (i & 4) {
            t.push(r);
          } else if (Kt(r)) {
            if ((i & p) !== 0) {
              this.add_dirty_reaction(r, w);
            }
            Jt(r);
          }
          const l = r.first;
          if (l !== null) {
            r = l;
            continue;
          }
        }
        while (r !== null) {
          const o = r.next;
          if (o !== null) {
            r = o;
            break;
          }
          r = r.parent;
        }
      }
    }
    #m() {
      for (let e = this.prev; e !== null; e = e.prev) {
        if (!e.is_fork && this.#b(e)) {
          return e;
        }
      }
      return null;
    }
    #b(e) {
      for (const [t, [, n]] of this.current) {
        if (!n && e.current.has(t)) {
          return true;
        }
      }
      return false;
    }
    #y(e) {
      for (const [t, n] of e.current) {
        if (!this.previous.has(t) && e.previous.has(t)) {
          this.previous.set(t, e.previous.get(t));
        }
        this.current.set(t, n);
      }
      for (const [t, n] of e.async_deriveds) {
        const e = this.async_deriveds.get(t);
        if (e) {
          n.promise.then(e.resolve).catch(e.reject);
        }
      }
      e.async_deriveds.clear();
      this.transfer_reactions(e.#o);
      const t = (e) => {
        const n = e.reactions;
        if (n !== null && (!(e.f & 2) || e.f & 6144)) {
          for (const e of n) {
            const r = e.f;
            if (r & 2) {
              t(e);
            } else {
              const i = e;
              if (r & 4194320 && !this.async_deriveds.has(i)) {
                this.#o.delete(i);
                te(i, b);
                this.schedule(i);
              }
            }
          }
        }
      };
      for (const e of this.current.keys()) {
        t(e);
      }
      this.oncommit(() => e.discard());
      e.#p();
      ye = this;
      this.#v();
    }
    #g(e) {
      for (let t = 0; t < e.length; t += 1) {
        re(e[t], this.#o);
      }
    }
    capture(e, t, n = false) {
      if (e.v !== K && !this.previous.has(e)) {
        this.previous.set(e, e.v);
      }
      if ((e.f & q) === 0) {
        this.current.set(e, [t, n]);
        we?.set(e, t);
      }
      if (!this.is_fork) {
        e.v = t;
      }
    }
    activate() {
      ye = this;
    }
    deactivate() {
      ye = null;
      we = null;
    }
    flush() {
      try {
        e;
        Se = true;
        ye = this;
        this.#v();
      } finally {
        Pe = 0;
        ke = null;
        je = null;
        Me = null;
        Se = false;
        ye = null;
        we = null;
        Re.clear();
      }
    }
    discard() {
      for (const e of this.#t) {
        e(this);
      }
      this.#t.clear();
      for (const e of this.async_deriveds.values()) {
        e.reject(ce);
      }
      this.#p();
      this.#i?.resolve();
    }
    register_created_effect(e) {
      this.#l.push(e);
    }
    #w() {
      for (let a = ge; a !== null; a = a.next) {
        const e = a.id < this.id;
        const t = [];
        for (const [r, [i, s]] of this.current) {
          if (a.current.has(r)) {
            const n = a.current.get(r)[0];
            if (!e || i === n) {
              continue;
            }
            a.current.set(r, [i, s]);
          }
          t.push(r);
        }
        if (e) {
          for (const [e, t] of this.async_deriveds) {
            const n = a.async_deriveds.get(e);
            if (n) {
              t.promise.then(n.resolve).catch(n.reject);
            }
          }
        }
        var r = [...a.current.keys()].filter((e) => !a.current.get(e)[1]);
        if (a.started && r.length !== 0) {
          const i = r.filter((e) => !this.current.has(e));
          if (i.length === 0) {
            if (e) {
              a.discard();
            }
          } else if (t.length > 0) {
            if (e) {
              for (const e of this.#u) {
                a.unskip_effect(e, (e) => {
                  if (e.f & 4194320) {
                    a.schedule(e);
                  } else {
                    a.#g([e]);
                  }
                });
              }
            }
            a.activate();
            const s = new Set();
            let l = new Map();
            for (const o of t) {
              Te(o, i, s, l);
            }
            l = new Map();
            const f = [...a.current].filter(([e, t]) => {
              const n = this.current.get(e);
              return !n || n[0] !== t[0] || n[1] !== t[1];
            }).map(([e]) => e);
            if (f.length > 0) {
              for (const e of this.#l) {
                if (!(e.f & 155648) && !!ze(e, f, l)) {
                  if (e.f & 4194320) {
                    te(e, b);
                    a.schedule(e);
                  } else {
                    a.add_dirty_reaction(e, b);
                  }
                }
              }
            }
            if (a.#s.length > 0 && !a.#a) {
              a.apply();
              for (const u of a.#h()) {
                a.#_(u, [], []);
              }
            }
            a.deactivate();
          }
        }
      }
    }
    increment(e, t) {
      this.#n += 1;
      if (e) {
        const e = this.#r.get(t) ?? 0;
        this.#r.set(t, e + 1);
      }
    }
    decrement(e, t) {
      this.#n -= 1;
      if (e) {
        const e = this.#r.get(t) ?? 0;
        if (e === 1) {
          this.#r.delete(t);
        } else {
          this.#r.set(t, e - 1);
        }
      }
      if (!this.#a) {
        this.#a = true;
        Y(() => {
          this.#a = false;
          if (this.linked) {
            this.flush();
          }
        });
      }
    }
    transfer_reactions(e) {
      for (const [t, n] of e) {
        this.add_dirty_reaction(t, n);
      }
      e.clear();
    }
    add_dirty_reaction(e, t) {
      if (t === b || this.#o.get(e) !== b) {
        this.#o.set(e, t);
      }
    }
    oncommit(e) {
      this.#e.add(e);
    }
    ondiscard(e) {
      this.#t.add(e);
    }
    settled() {
      return (this.#i ??= h()).promise;
    }
    static ensure() {
      if (ye === null) {
        const e = ye = new $e();
        if (!Se && !xe) {
          Y(() => {
            if (!e.started) {
              e.flush();
            }
          });
        }
      }
      return ye;
    }
    apply() {
      we = null;
    }
    schedule(e) {
      ke = e;
      if (e.b?.is_pending && e.f & 16777228 && (e.f & S) === 0) {
        e.b.defer_effect(e);
      } else {
        this.#s.push(e);
      }
    }
    #p() {
      if (this.linked) {
        const e = this.prev;
        const t = this.next;
        if (e === null) {
          ge = t;
        } else {
          e.next = t;
        }
        if (t === null) {
          me = e;
        } else {
          t.prev = e;
        }
        this.linked = false;
      }
    }
  }
  function Oe() {
    try {
      (function () {
        throw new Error("https://svelte.dev/e/effect_update_depth_exceeded");
      })();
    } catch (e) {
      ft(e, ke);
    }
  }
  let qe = null;
  function Ae(e) {
    const t = e.length;
    if (t !== 0) {
      for (let n = 0; n < t;) {
        const r = e[n++];
        if (!(r.f & 24576) && Kt(r) && (qe = new Set(), Jt(r), r.deps === null && r.first === null && r.nodes === null && r.teardown === null && r.ac === null && yt(r), qe?.size > 0)) {
          Re.clear();
          for (const e of qe) {
            if (e.f & 24576) {
              continue;
            }
            const t = [e];
            let n = e.parent;
            while (n !== null) {
              if (qe.has(n)) {
                qe.delete(n);
                t.push(n);
              }
              n = n.parent;
            }
            for (let e = t.length - 1; e >= 0; e--) {
              const n = t[e];
              if (!(n.f & 24576)) {
                Jt(n);
              }
            }
          }
          qe.clear();
        }
      }
      qe = null;
    }
  }
  function Te(e, t, n, r) {
    if (!n.has(e) && (n.add(e), e.reactions !== null)) {
      for (const i of e.reactions) {
        const e = i.f;
        if (e & 2) {
          Te(i, t, n, r);
        } else if (e & 4194320 && (e & b) === 0 && ze(i, t, r)) {
          te(i, b);
          Ce(i);
        }
      }
    }
  }
  function ze(e, t, n) {
    const i = n.get(e);
    if (i !== undefined) {
      return i;
    }
    if (e.deps !== null) {
      for (const i of e.deps) {
        if (r.call(t, i)) {
          return true;
        }
        if (i.f & 2 && ze(i, t, n)) {
          n.set(i, true);
          return true;
        }
      }
    }
    n.set(e, false);
    return false;
  }
  function Ce(e) {
    ye.schedule(e);
  }
  function De(e, t) {
    if ((e.f & _) === 0 || (e.f & y) === 0) {
      if ((e.f & b) !== 0) {
        t.d.push(e);
      } else if ((e.f & w) !== 0) {
        t.m.push(e);
      }
      te(e, y);
      for (let n = e.first; n !== null;) {
        De(n, t);
        n = n.next;
      }
    }
  }
  function Ne(e) {
    te(e, y);
    for (let t = e.first; t !== null;) {
      Ne(t);
      t = t.next;
    }
  }
  const Le = new Set();
  const Re = new Map();
  let Fe = false;
  function He(e, t) {
    return {
      f: 0,
      v: e,
      reactions: null,
      equals: V,
      rv: 0,
      wv: 0
    };
  }
  function Ke(e, t) {
    const n = He(e);
    Tt(n);
    return n;
  }
  function Ve(e, t, n = false) {
    if (Pt !== null && (!Et || (Pt.f & P) !== 0) && !!(Pt.f & 4325394) && (At === null || !At.has(e))) {
      (function () {
        throw new Error("https://svelte.dev/e/state_unsafe_mutation");
      })();
    }
    return Ue(e, n ? Ze(t) : t, Me);
  }
  var Je = null;
  var Qe = 0;
  function Ue(e, t, n = null) {
    if (!e.equals(t)) {
      if (jt) {
        Re.set(e, t);
      } else if (!Re.has(e)) {
        Re.set(e, e.v);
      }
      const r = $e.ensure();
      r.capture(e, t);
      if (e.f & 2) {
        const t = e;
        if ((e.f & b) !== 0) {
          ve(t);
        }
        if (we === null) {
          ne(t);
        }
      }
      e.wv = Ht();
      Je = null;
      Qe = 0;
      Ye(e, b, n);
      Je = null;
      if (Ot !== null && (Ot.f & y) !== 0 && !(Ot.f & 96)) {
        if (Dt === null) {
          (function (e) {
            Dt = e;
          })([e]);
        } else {
          Dt.push(e);
        }
      }
      if (!r.is_fork && Le.size > 0 && !Fe) {
        (function () {
          Fe = false;
          for (const e of Le) {
            let t;
            if ((e.f & y) !== 0) {
              te(e, w);
            }
            try {
              t = Kt(e);
            } catch {
              t = true;
            }
            if (t) {
              Jt(e);
            }
          }
          Le.clear();
        })();
      }
    }
    return t;
  }
  function Xe(e) {
    Ve(e, e.v + 1);
  }
  function Ye(e, t, n) {
    const r = e.reactions;
    if (r !== null) {
      const i = r.length;
      if ((Qe += i) > 1e5 && Je === null) {
        Je = new Set();
      }
      if (Je !== null) {
        if (Je.has(e)) {
          return;
        }
        Je.add(e);
      }
      for (let s = 0; s < i; s++) {
        const l = r[s];
        const o = l.f;
        const f = (o & b) === 0;
        if (f) {
          te(l, t);
        }
        if ((o & P) !== 0) {
          Le.add(l);
        } else if (o & 2) {
          const u = l;
          we?.delete(u);
          Ye(u, w, n);
        } else if (f) {
          const a = l;
          if ((o & p) !== 0 && qe !== null) {
            qe.add(a);
          }
          if (n !== null) {
            n.push(a);
          } else {
            Ce(a);
          }
        }
      }
    }
  }
  function Ze(e) {
    if (typeof e != "object" || e === null || A in e || T in e) {
      return e;
    }
    const n = u(e);
    if (n !== o && n !== f) {
      return e;
    }
    const r = new Map();
    const i = t(e);
    const s = Ke(0);
    const a = Rt;
    const c = (e) => {
      if (Rt === a) {
        return e();
      }
      const t = Pt;
      const n = Rt;
      $t(null);
      Ft(a);
      const r = e();
      $t(t);
      Ft(n);
      return r;
    };
    if (i) {
      r.set("length", Ke(e.length));
    }
    return new Proxy(e, {
      defineProperty(e, t, n) {
        if (!("value" in n) || n.configurable === false || n.enumerable === false || n.writable === false) {
          (function () {
            throw new Error("https://svelte.dev/e/state_descriptors_fixed");
          })();
        }
        const i = r.get(t);
        if (i === undefined) {
          c(() => {
            const e = Ke(n.value);
            r.set(t, e);
            return e;
          });
        } else {
          Ve(i, n.value, true);
        }
        return true;
      },
      deleteProperty(e, t) {
        const n = r.get(t);
        if (n === undefined) {
          if (t in e) {
            const e = c(() => Ke(K));
            r.set(t, e);
            Xe(s);
          }
        } else {
          Ve(n, K);
          Xe(s);
        }
        return true;
      },
      get(t, n, i) {
        if (n === A) {
          return e;
        }
        let s = r.get(n);
        const o = n in t;
        if (s === undefined && (!o || !!l(t, n)?.writable)) {
          s = c(() => Ke(Ze(o ? t[n] : K)));
          r.set(n, s);
        }
        if (s !== undefined) {
          const f = Ut(s);
          if (f === K) {
            return undefined;
          } else {
            return f;
          }
        }
        return Reflect.get(t, n, i);
      },
      getOwnPropertyDescriptor(e, t) {
        this.has?.(e, t);
        const n = Reflect.getOwnPropertyDescriptor(e, t);
        const i = r.get(t);
        if (i !== undefined) {
          const s = Ut(i);
          if (s === K) {
            return;
          }
          if (!n || !("value" in n)) {
            return {
              enumerable: true,
              configurable: true,
              value: s,
              writable: true
            };
          }
          n.value = s;
        }
        return n;
      },
      has(e, t) {
        if (t === A) {
          return true;
        }
        let n = r.get(t);
        const i = n !== undefined && n.v !== K || Reflect.has(e, t);
        if ((n !== undefined || (Ot !== null || Pt !== null) && (!i || l(e, t)?.writable)) && (n === undefined && (n = c(() => Ke(i ? Ze(e[t]) : K)), r.set(t, n)), Ut(n) === K)) {
          return false;
        }
        return i;
      },
      set(e, t, n, o) {
        let f = r.get(t);
        let u = t in e;
        if (i && t === "length") {
          for (let a = n; a < f.v; a += 1) {
            let d = r.get(a + "");
            if (d !== undefined) {
              Ve(d, K);
            } else if (a in e) {
              d = c(() => Ke(K));
              r.set(a + "", d);
            }
          }
        }
        if (f === undefined) {
          if (!u || !!l(e, t)?.writable) {
            Ve(f = c(() => Ke(undefined)), Ze(n));
            r.set(t, f);
          }
        } else {
          u = f.v !== K;
          Ve(f, c(() => Ze(n)));
        }
        const h = Reflect.getOwnPropertyDescriptor(e, t);
        if (h?.set) {
          h.set.call(o, n);
        }
        if (!u) {
          if (i && typeof t == "string") {
            const v = r.get("length");
            const p = Number(t);
            if (Number.isInteger(p) && p >= v.v) {
              Ve(v, p + 1);
            }
          }
          Xe(s);
        }
        return true;
      },
      ownKeys(e) {
        Ut(s);
        const t = Reflect.ownKeys(e).filter((e) => {
          const t = r.get(e);
          return t === undefined || t.v !== K;
        });
        for (var [n, i] of r) {
          if (i.v !== K && !(n in e)) {
            t.push(n);
          }
        }
        return t;
      },
      setPrototypeOf() {
        (function () {
          throw new Error("https://svelte.dev/e/state_prototype_fixed");
        })();
      }
    });
  }
  function et(e = "") {
    return document.createTextNode(e);
  }
  var Ie;
  function tt(e) {
    return Ie.call(e);
  }
  var Ge;
  function nt(e) {
    return Ge.call(e);
  }
  function rt(e, t) {
    return tt(e);
  }
  function it(e, t = false) {
    const n = tt(e);
    if (n instanceof Comment && n.data === "") {
      return nt(n);
    } else {
      return n;
    }
  }
  function st(e, t = false) {
    return tt(e);
  }
  function lt(e, t = 1, n = false) {
    let r = e;
    while (t--) {
      r = nt(r);
    }
    return r;
  }
  function ot(e, t, n) {
    if (n) {
      return document.createElement(e, { is: n });
    } else {
      return document.createElement(e);
    }
  }
  function ft(e, t) {
    if (e === H) {
      throw e;
    }
    if (t === null || (t.f & x) === 0) {
      while (t !== null) {
        if (t.f & 128 && !(t.f & 33570816)) {
          if ((t.f & S) === 0) {
            throw e;
          }
          try {
            t.b.error(e);
            return;
          } catch (t) {
            e = t;
          }
        }
        t = t.parent;
      }
      throw e;
    }
  }
  function ut(e, t) {
    const n = Ot;
    if (n !== null && (n.f & k) !== 0) {
      e |= k;
    }
    const r = {
      ctx: B,
      deps: null,
      nodes: null,
      f: e | b | m,
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
    ye?.register_created_effect(r);
    let i = r;
    if (e & 4) {
      if (je !== null) {
        je.push(r);
      } else {
        $e.ensure().schedule(r);
      }
    } else if (t !== null) {
      try {
        Jt(r);
      } catch (i) {
        mt(r);
        throw i;
      }
      if (i.deps === null && i.teardown === null && i.nodes === null && i.first === i.last && (i.f & E) === 0) {
        i = i.first;
        if ((e & p) !== 0 && (e & M) !== 0 && i !== null) {
          i.f |= M;
        }
      }
    }
    if (i !== null && (i.parent = n, n !== null && function (e, t) {
      const n = t.last;
      if (n === null) {
        t.last = t.first = e;
      } else {
        n.next = e;
        e.prev = n;
        t.last = e;
      }
    }(i, n), Pt !== null && Pt.f & 2 && (e & g) === 0)) {
      const s = Pt;
      (s.effects ??= []).push(i);
    }
    return r;
  }
  function at() {
    return Pt !== null && !Et;
  }
  function ct(e) {
    return ut(1048580, e);
  }
  function dt(e, t = 0) {
    return ut(t | 8, e);
  }
  function ht(e, t = [], n = [], r = []) {
    oe(r, t, n, (t) => {
      ut(8, () => {
        e(...t.map(Ut));
      });
    });
  }
  function vt(e, t = 0) {
    return ut(p | t, e);
  }
  function pt(e) {
    return ut(524320, e);
  }
  function _t(e) {
    const t = e.teardown;
    if (t !== null) {
      const n = jt;
      const r = Pt;
      Mt(true);
      $t(null);
      try {
        t.call(null);
      } catch (t) {
        ft(t, e.parent);
      } finally {
        Mt(n);
        $t(r);
      }
    }
  }
  function gt(e, t = false) {
    let n = e.first;
    for (e.first = e.last = null; n !== null;) {
      const e = n.ac;
      if (e !== null) {
        se(() => {
          e.abort(F);
        });
      }
      const r = n.next;
      if ((n.f & g) !== 0) {
        n.parent = null;
      } else {
        mt(n, t);
      }
      n = r;
    }
  }
  function mt(e, t = true) {
    let n = false;
    if ((t || e.f & 262144) && e.nodes !== null && e.nodes.end !== null) {
      (function (e, t) {
        while (e !== null) {
          const n = e === t ? null : nt(e);
          e.remove();
          e = n;
        }
      })(e.nodes.start, e.nodes.end);
      n = true;
    }
    e.f |= j;
    gt(e, t && !n);
    Gt(e, 0);
    const r = e.nodes && e.nodes.t;
    if (r !== null) {
      for (const e of r) {
        e.stop();
      }
    }
    _t(e);
    e.f ^= j;
    e.f |= x;
    const i = e.parent;
    if (i !== null && i.first !== null) {
      yt(e);
    }
    e.next = e.prev = e.teardown = e.ctx = e.deps = e.fn = e.nodes = e.ac = e.b = null;
  }
  function yt(e) {
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
  function bt(e, t, n = true) {
    const r = [];
    e.f |= 256;
    wt(e, r, true);
    const i = () => {
      if (n) {
        mt(e);
      }
      if (t) {
        t();
      }
    };
    let s = r.length;
    if (s > 0) {
      const l = () => --s || i();
      for (const o of r) {
        o.out(l);
      }
    } else {
      i();
    }
  }
  function wt(e, t, n) {
    if ((e.f & k) === 0) {
      e.f ^= k;
      const r = e.nodes && e.nodes.t;
      if (r !== null) {
        for (const e of r) {
          if (e.is_global || n) {
            t.push(e);
          }
        }
      }
      for (let i = e.first; i !== null;) {
        const s = i.next;
        if ((i.f & g) === 0) {
          wt(i, t, ((i.f & M) !== 0 || (i.f & _) !== 0 && (e.f & p) !== 0) && n);
        }
        i = s;
      }
    }
  }
  function kt(e) {
    e.f &= -257;
    xt(e, true);
  }
  function xt(e, t) {
    if (!(e.f & 256) && (e.f & k) !== 0) {
      e.f ^= k;
      if ((e.f & y) === 0) {
        te(e, b);
        $e.ensure().schedule(e);
      }
      for (let n = e.first; n !== null;) {
        const r = n.next;
        xt(n, ((n.f & M) !== 0 || (n.f & _) !== 0) && t);
        n = r;
      }
      const i = e.nodes && e.nodes.t;
      if (i !== null) {
        for (const e of i) {
          if (e.is_global || t) {
            e.in();
          }
        }
      }
    }
  }
  function St(e, t) {
    if (e.nodes) {
      for (let n = e.nodes.start, r = e.nodes.end; n !== null;) {
        const i = n === r ? null : nt(n);
        t.append(n);
        n = i;
      }
    }
  }
  let jt = false;
  function Mt(e) {
    jt = e;
  }
  let Pt = null;
  let Et = false;
  function $t(e) {
    Pt = e;
  }
  let Ot = null;
  function qt(e) {
    Ot = e;
  }
  let At = null;
  function Tt(e) {
    if (Pt !== null && ((Pt.f & O) !== 0 || Pt.f & 2)) {
      (At ??= new Set()).add(e);
    }
  }
  let zt = null;
  let Ct = 0;
  let Dt = null;
  let Nt = 1;
  let Lt = 0;
  let Rt = Lt;
  function Ft(e) {
    Rt = e;
  }
  function Ht() {
    return ++Nt;
  }
  function Kt(e) {
    const t = e.f;
    if ((t & b) !== 0) {
      return true;
    }
    if ((t & w) !== 0) {
      const n = e.deps;
      for (let r = n.length, i = 0; i < r; i++) {
        const s = n[i];
        if (Kt(s)) {
          pe(s);
        }
        if (s.wv > e.wv) {
          return true;
        }
      }
      if ((t & m) !== 0 && we === null) {
        te(e, y);
      }
    }
    return false;
  }
  function Vt(e, t, n = true) {
    const r = e.reactions;
    if (r !== null && (At === null || !At.has(e))) {
      for (let i = 0; i < r.length; i++) {
        const s = r[i];
        if (s.f & 2) {
          Vt(s, t, false);
        } else if (t === s) {
          if (n) {
            te(s, b);
          } else if ((s.f & y) !== 0) {
            te(s, w);
          }
          Ce(s);
        }
      }
    }
  }
  function Wt(e) {
    const t = zt;
    const n = Ct;
    let r = Dt;
    const i = Pt;
    const s = At;
    const l = B;
    const o = Et;
    const f = Rt;
    const u = e.f;
    zt = null;
    Ct = 0;
    Dt = null;
    Pt = u & 96 ? null : e;
    At = null;
    I(e.ctx);
    Et = false;
    Rt = ++Lt;
    if (e.ac !== null) {
      se(() => {
        e.ac.abort(F);
      });
      e.ac = null;
    }
    try {
      e.f |= O;
      const a = (0, e.fn)();
      e.f |= S;
      const c = Bt(e);
      if (Dt !== null && !Et && c !== null && !(e.f & 6146)) {
        for (let d = 0; d < Dt.length; d++) {
          Vt(Dt[d], e);
        }
      }
      if (i !== null && i !== e) {
        Lt++;
        if (i.deps !== null) {
          for (let e = 0; e < n; e += 1) {
            i.deps[e].rv = Lt;
          }
        }
        if (t !== null) {
          for (const e of t) {
            e.rv = Lt;
          }
        }
        if (Dt !== null) {
          if (r === null) {
            r = Dt;
          } else {
            r.push(...Dt);
          }
        }
      }
      if ((e.f & q) !== 0) {
        e.f ^= q;
      }
      return a;
    } catch (t) {
      Bt(e);
      return function (e) {
        const t = Ot;
        if (t === null) {
          Pt.f |= q;
          return e;
        }
        if ((t.f & S) === 0 && !(t.f & 4)) {
          throw e;
        }
        ft(e, t);
      }(t);
    } finally {
      e.f ^= O;
      zt = t;
      Ct = n;
      Dt = r;
      Pt = i;
      At = s;
      I(l);
      Et = o;
      Rt = f;
    }
  }
  function Bt(e) {
    let t = e.deps;
    const n = ye?.is_fork;
    if (zt !== null) {
      var r;
      if (!n) {
        Gt(e, Ct);
      }
      if (t !== null && Ct > 0) {
        t.length = Ct + zt.length;
        r = 0;
        for (; r < zt.length; r++) {
          t[Ct + r] = zt[r];
        }
      } else {
        e.deps = t = zt;
      }
      if (at() && (e.f & m) !== 0) {
        for (r = Ct; r < t.length; r++) {
          (t[r].reactions ??= []).push(e);
        }
      }
    } else if (!n && t !== null && Ct < t.length) {
      Gt(e, Ct);
      t.length = Ct;
    }
    return t;
  }
  function It(e, t) {
    let i = t.reactions;
    if (i !== null) {
      const s = n.call(i, e);
      if (s !== -1) {
        const l = i.length - 1;
        if (l === 0) {
          i = t.reactions = null;
        } else {
          i[s] = i[l];
          i.pop();
        }
      }
    }
    if (i === null && t.f & 2 && (zt === null || !r.call(zt, t))) {
      var o = t;
      if ((o.f & m) !== 0) {
        o.f ^= m;
      }
      if (o.v !== K) {
        ne(o);
      }
      if (o.ac !== null) {
        se(() => {
          o.ac.abort(F);
          o.ac = null;
          te(o, b);
        });
      }
      (function (e) {
        if (e.effects !== null) {
          for (const t of e.effects) {
            if (t.teardown || t.ac) {
              t.teardown?.();
              if (t.ac !== null) {
                se(() => {
                  t.ac.abort(F);
                  t.ac = null;
                });
              }
              if (t.fn !== null) {
                t.teardown = d;
              }
              Gt(t, 0);
              gt(t);
            }
          }
        }
      })(o);
      Gt(o, 0);
    }
  }
  function Gt(e, t) {
    const n = e.deps;
    if (n !== null) {
      for (let r = t; r < n.length; r++) {
        It(e, n[r]);
      }
    }
  }
  function Jt(e) {
    const t = e.f;
    if ((t & x) === 0) {
      te(e, y);
      const n = Ot;
      Ot = e;
      try {
        if (t & 16777232) {
          (function (e) {
            for (let t = e.first; t !== null;) {
              const n = t.next;
              if ((t.f & _) === 0) {
                mt(t);
              }
              t = n;
            }
          })(e);
        } else {
          gt(e);
        }
        _t(e);
        const r = Wt(e);
        e.teardown = typeof r == "function" ? r : null;
        e.wv = Nt;
      } finally {
        Ot = n;
      }
    }
  }
  async function Qt() {
    await Promise.resolve();
    (function () {
      const e = xe;
      const t = be;
      be = null;
      xe = true;
      try {
        while (true) {
          Z();
          if (ye === null) {
            return;
          }
          ye.flush();
        }
      } finally {
        xe = e;
        be = t;
      }
    })();
  }
  function Ut(e) {
    const t = !!(e.f & 2);
    if (Pt !== null && !Et && (Ot === null || (Ot.f & x) === 0) && (At === null || !At.has(e))) {
      const n = Pt.deps;
      if ((Pt.f & O) !== 0) {
        if (e.rv < Lt) {
          e.rv = Lt;
          if (zt === null && n !== null && n[Ct] === e) {
            Ct++;
          } else if (zt === null) {
            zt = [e];
          } else {
            zt.push(e);
          }
        }
      } else {
        Pt.deps ??= [];
        if (!r.call(Pt.deps, e)) {
          Pt.deps.push(e);
        }
        const i = e.reactions;
        if (i === null) {
          e.reactions = [Pt];
        } else if (!r.call(i, Pt)) {
          i.push(Pt);
        }
      }
    }
    if (jt && Re.has(e)) {
      return Re.get(e);
    }
    if (t) {
      const s = e;
      if (jt) {
        let l = s.v;
        if ((s.f & y) === 0 && s.reactions !== null || Yt(s)) {
          l = ve(s);
        }
        Re.set(s, l);
        return l;
      }
      const o = (s.f & m) === 0 && !Et && Pt !== null && (Pt.f & m) !== 0;
      const f = (s.f & S) === 0;
      if (Kt(s)) {
        if (o) {
          s.f |= m;
        }
        pe(s);
      }
      if (o && !f) {
        _e(s);
        Xt(s);
      }
    }
    if (we?.has(e)) {
      return we.get(e);
    }
    if ((e.f & q) !== 0) {
      throw e.v;
    }
    return e.v;
  }
  function Xt(e) {
    e.f |= m;
    if (e.deps !== null) {
      for (const n of e.deps) {
        const t = n.reactions;
        if (t === null) {
          n.reactions = [e];
        } else if (!r.call(t, e)) {
          t.push(e);
        }
        if (n.f & 2 && (n.f & m) === 0) {
          _e(n);
          Xt(n);
        }
      }
    }
  }
  function Yt(e) {
    if (e.v === K) {
      return true;
    }
    if (e.deps === null) {
      return false;
    }
    for (const t of e.deps) {
      if (Re.has(t)) {
        return true;
      }
      if (t.f & 2 && Yt(t)) {
        return true;
      }
    }
    return false;
  }
  function Zt(e) {
    const t = Et;
    try {
      Et = true;
      return e();
    } finally {
      Et = t;
    }
  }
  const en = ["touchstart", "touchmove"];
  function tn(e) {
    return en.includes(e);
  }
  const nn = Symbol("events");
  const rn = new Set();
  const sn = new Set();
  function ln(e, t, n) {
    (t[nn] ??= {})[e] = n;
  }
  function on(e) {
    for (let t = 0; t < e.length; t++) {
      rn.add(e[t]);
    }
    for (const n of sn) {
      n(e);
    }
  }
  let fn = null;
  let un = false;
  function an(e) {
    const t = this;
    const n = t.ownerDocument;
    const r = e.type;
    const i = e.composedPath?.() || [];
    let l = i[0] || e.target;
    fn = e;
    if (!un) {
      un = true;
      setTimeout(() => {
        un = false;
        fn = null;
      });
    }
    let o = 0;
    const f = fn === e && e[nn];
    if (f) {
      const u = i.indexOf(f);
      if (u !== -1 && (t === document || t === window)) {
        e[nn] = t;
        return;
      }
      const a = i.indexOf(t);
      if (a === -1) {
        return;
      }
      if (u <= a) {
        o = u;
      }
    }
    if ((l = i[o] || e.target) !== t) {
      s(e, "currentTarget", {
        configurable: true,
        get: () => l || n
      });
      const c = Pt;
      const d = Ot;
      $t(null);
      qt(null);
      try {
        var h;
        const v = [];
        while (l !== null && l !== t) {
          try {
            const p = l[nn]?.[r];
            if (p != null && (!l.disabled || e.target === l)) {
              p.call(l, e);
            }
          } catch (e) {
            if (h) {
              v.push(e);
            } else {
              h = e;
            }
          }
          if (e.cancelBubble) {
            break;
          }
          o++;
          l = o < i.length ? i[o] : null;
        }
        if (h) {
          for (let e of v) {
            queueMicrotask(() => {
              throw e;
            });
          }
          throw h;
        }
      } finally {
        e[nn] = t;
        delete e.currentTarget;
        $t(c);
        qt(d);
      }
    }
  }
  const cn = globalThis?.window?.trustedTypes && globalThis.window.trustedTypes.createPolicy("svelte-trusted-html", { createHTML: (e) => e });
  function dn(e) {
    const t = ot("template");
    t.innerHTML = function (e) {
      return cn?.createHTML(e) ?? e;
    }(e.replaceAll("<!>", "<!---->"));
    return t.content;
  }
  function hn(e, t) {
    const n = Ot;
    if (n.nodes === null) {
      n.nodes = {
        start: e,
        end: t,
        a: null,
        t: null
      };
    }
  }
  var Be;
  function vn(e, t) {
    const r = !!(t & 1);
    const i = !!(t & 2);
    const s = !e.startsWith("<!>");
    let n;
    return () => {
      if (n === undefined) {
        n = dn(s ? e : "<!>" + e);
        if (!r) {
          n = tt(n);
        }
      }
      const t = i || Be ? document.importNode(n, true) : n.cloneNode(true);
      if (r) {
        hn(tt(t), t.lastChild);
      } else {
        hn(t, t);
      }
      return t;
    };
  }
  function pn(e, t) {
    if (e !== null) {
      e.before(t);
    }
  }
  function _n(e, t, n, r) {
    new gn(e, t, n, r);
  }
  const gn = class {
    parent;
    is_pending = false;
    transform_error;
    #k;
    #x = null;
    #S;
    #j;
    #M;
    #P = null;
    #E = null;
    #$ = null;
    #O = null;
    #q = 0;
    #A = 0;
    #T = false;
    #o = new Map();
    #z = null;
    #C = function (e) {
      let t;
      let n = 0;
      const r = He(0);
      return () => {
        if (at()) {
          Ut(r);
          dt(() => {
            if (n === 0) {
              t = Zt(() => e(() => Xe(r)));
            }
            n += 1;
            return () => {
              Y(() => {
                n -= 1;
                if (n === 0) {
                  t?.();
                  t = undefined;
                  Xe(r);
                }
              });
            };
          });
        }
      };
    }(() => {
      this.#z = He(this.#q);
      return () => {
        this.#z = null;
      };
    });
    constructor(e, t, n, r) {
      this.#k = e;
      this.#S = t;
      this.#j = (e) => {
        const t = Ot;
        t.b = this;
        t.f |= 128;
        n(e);
      };
      this.parent = Ot.b;
      this.transform_error = r ?? this.parent?.transform_error ?? ((e) => e);
      this.#M = vt(() => {
        this.#D();
      }, 589824);
    }
    #N() {
      try {
        this.#P = pt(() => this.#j(this.#k));
      } catch (e) {
        this.error(e);
      }
    }
    #L(e) {
      const t = this.#S.failed;
      const { reset: n, invoke_onerror: r } = this.#R(e);
      Y(r);
      if (t) {
        this.#$ = pt(() => {
          t(this.#k, () => e, () => n);
        });
      }
    }
    #R(e) {
      let t = false;
      let n = false;
      const r = () => {
        if (!this.#F()) {
          if (t) {
            console.warn("https://svelte.dev/e/svelte_boundary_reset_noop");
          } else {
            t = true;
            if (n) {
              (function () {
                throw new Error("https://svelte.dev/e/svelte_boundary_reset_onerror");
              })();
            }
            if (this.#$ !== null) {
              bt(this.#$, () => {
                this.#$ = null;
              });
            }
            this.#H(() => {
              this.#D();
            });
          }
        }
      };
      return {
        reset: r,
        invoke_onerror: () => {
          if (!this.#F()) {
            try {
              n = true;
              this.#S.onerror?.(e, r);
              n = false;
            } catch (e) {
              ft(e, this.#M && this.#M.parent);
            }
          }
        }
      };
    }
    #F() {
      return !!(this.#M.f & 33570816);
    }
    #K() {
      const e = this.#S.pending;
      if (e) {
        this.is_pending = true;
        this.#E = pt(() => e(this.#k));
        Y(() => {
          if (!this.#F()) {
            const e = this.#O = document.createDocumentFragment();
            var t = et();
            var n = false;
            e.append(t);
            this.#P = this.#H(() => {
              try {
                return pt(() => this.#j(t));
              } catch (e) {
                try {
                  this.error(e);
                  n = true;
                } catch (e) {
                  ft(e, this.#M.parent);
                }
                return null;
              }
            });
            if (this.#P === null) {
              this.#O = null;
              if (n) {
                this.#h(ye);
              }
              return;
            }
            if (this.#A === 0) {
              this.#k.before(e);
              this.#O = null;
              bt(this.#E, () => {
                this.#E = null;
              });
              this.#h(ye);
            }
          }
        });
      }
    }
    #D() {
      try {
        this.is_pending = this.has_pending_snippet();
        this.#A = 0;
        this.#q = 0;
        this.#P = pt(() => {
          this.#j(this.#k);
        });
        if (this.#A > 0) {
          const e = this.#O = document.createDocumentFragment();
          St(this.#P, e);
          const t = this.#S.pending;
          this.#E = pt(() => t(this.#k));
        } else {
          this.#h(ye);
        }
      } catch (e) {
        this.error(e);
      }
    }
    #h(e) {
      this.is_pending = false;
      e.transfer_reactions(this.#o);
    }
    defer_effect(e) {
      re(e, this.#o);
    }
    is_rendered() {
      return !this.is_pending && (!this.parent || this.parent.is_rendered());
    }
    has_pending_snippet() {
      return !!this.#S.pending;
    }
    #H(e) {
      const t = Ot;
      const n = Pt;
      const r = B;
      qt(this.#M);
      $t(this.#M);
      I(this.#M.ctx);
      try {
        $e.ensure();
        return e();
      } finally {
        qt(t);
        $t(n);
        I(r);
      }
    }
    #V(e, t) {
      if (this.has_pending_snippet()) {
        this.#A += e;
        if (this.#A === 0) {
          this.#h(t);
          if (this.#E) {
            bt(this.#E, () => {
              this.#E = null;
            });
          }
          if (this.#O) {
            this.#k.before(this.#O);
            this.#O = null;
          }
        }
      } else if (this.parent) {
        this.parent.#V(e, t);
      }
    }
    update_pending_count(e, t) {
      this.#V(e, t);
      this.#q += e;
      if (this.#z && !this.#T) {
        this.#T = true;
        Y(() => {
          this.#T = false;
          if (this.#z) {
            Ue(this.#z, this.#q);
          }
        });
      }
    }
    get_effect_pending() {
      this.#C();
      return Ut(this.#z);
    }
    error(e) {
      if (e === H) {
        throw e;
      }
      if (!this.#S.onerror && !this.#S.failed) {
        throw e;
      }
      if (ye?.is_fork) {
        if (this.#P) {
          ye.skip_effect(this.#P);
        }
        if (this.#E) {
          ye.skip_effect(this.#E);
        }
        if (this.#$) {
          ye.skip_effect(this.#$);
        }
        ye.oncommit(() => {
          if (!this.#F()) {
            this.#W(e);
          }
        });
      } else {
        this.#W(e);
      }
    }
    #W(e) {
      if (this.#P) {
        mt(this.#P);
        this.#P = null;
      }
      if (this.#E) {
        mt(this.#E);
        this.#E = null;
      }
      if (this.#$) {
        mt(this.#$);
        this.#$ = null;
      }
      const t = this.#S.failed;
      const n = (e) => {
        if (this.#F()) {
          return;
        }
        const { reset: n, invoke_onerror: r } = this.#R(e);
        r();
        if (t && !this.#F()) {
          this.#$ = this.#H(() => {
            try {
              return pt(() => {
                const r = Ot;
                r.b = this;
                r.f |= 128;
                t(this.#k, () => e, () => n);
              });
            } catch (e) {
              ft(e, this.#M.parent);
              return null;
            }
          });
        }
      };
      Y(() => {
        if (!this.#F()) {
          var t;
          try {
            t = this.transform_error(e);
          } catch (e) {
            ft(e, this.#M && this.#M.parent);
            return;
          }
          if (t !== null && typeof t == "object" && typeof t.then == "function") {
            t.then(n, (e) => ft(e, this.#M && this.#M.parent));
          } else {
            n(t);
          }
        }
      });
    }
  };
  function mn(e, t) {
    const n = t == null ? "" : typeof t == "object" ? `${t}` : t;
    if (n !== (e[L] ??= e.nodeValue)) {
      e[L] = n;
      e.nodeValue = `${n}`;
    }
  }
  const yn = new Map();
  const bn = new WeakMap();
  class wn {
    anchor;
    #B = new Map();
    #I = new Map();
    #G = new Map();
    #J = new Set();
    #Q = true;
    constructor(e, t = true) {
      this.anchor = e;
      this.#Q = t;
    }
    #w = (e) => {
      if (this.#B.has(e)) {
        const t = this.#B.get(e);
        let n = this.#I.get(t);
        if (n) {
          kt(n);
          this.#J.delete(t);
        } else {
          const r = this.#G.get(t);
          if (r) {
            kt(r.effect);
            this.#I.set(t, r.effect);
            this.#G.delete(t);
            r.fragment.lastChild.remove();
            this.anchor.before(r.fragment);
            n = r.effect;
          }
        }
        for (const [t, n] of this.#B) {
          this.#B.delete(t);
          if (t === e) {
            break;
          }
          const r = this.#G.get(n);
          if (r) {
            mt(r.effect);
            this.#G.delete(n);
          }
        }
        for (const [e, r] of this.#I) {
          if (e === t || this.#J.has(e)) {
            continue;
          }
          const i = () => {
            if (Array.from(this.#B.values()).includes(e)) {
              const t = document.createDocumentFragment();
              St(r, t);
              t.append(et());
              this.#G.set(e, {
                effect: r,
                fragment: t
              });
            } else {
              mt(r);
            }
            this.#J.delete(e);
            this.#I.delete(e);
          };
          if (this.#Q || !n) {
            this.#J.add(e);
            bt(r, i, false);
          } else {
            i();
          }
        }
      }
    };
    #U = (e) => {
      this.#B.delete(e);
      const t = Array.from(this.#B.values());
      for (const [e, n] of this.#G) {
        if (!t.includes(e)) {
          mt(n.effect);
          this.#G.delete(e);
        }
      }
    };
    ensure(e, t) {
      const n = ye;
      if (!!t && !this.#I.has(e) && !this.#G.has(e)) {
        this.#I.set(e, pt(() => t(this.anchor)));
      }
      this.#B.set(n, e);
      this.#w(n);
    }
  }
  function kn(e, t) {
    return t;
  }
  function xn(e, t, n = true) {
    let r;
    if (e.pending.size > 0) {
      r = new Set();
      for (const t of e.pending.values()) {
        for (const n of t) {
          r.add(e.items.get(n).e);
        }
      }
    }
    for (let i = 0; i < t.length; i++) {
      const s = t[i];
      if (r?.has(s)) {
        s.f |= $;
        St(s, document.createDocumentFragment());
      } else {
        mt(t[i], n);
      }
    }
  }
  var Sn;
  function jn(e, n, r, s, l, o = null) {
    var f = e;
    const u = new Map();
    if (n & 4) {
      f = e.appendChild(et());
    }
    var a = null;
    var c = he(() => {
      const e = r();
      if (t(e)) {
        return e;
      } else if (e == null) {
        return [];
      } else {
        return i(e);
      }
    });
    const d = new Map();
    let h = true;
    function v(e) {
      if ((_.effect.f & x) === 0) {
        _.pending.delete(e);
        const t = Ut(c);
        _.fallback = a;
        (function (e, t, n, r, s) {
          const c = !!(r & 8);
          const d = t.length;
          const h = e.items;
          let v = Mn(e.effect.first);
          let p = null;
          let _ = [];
          let g = [];
          let o;
          let f;
          let u;
          let a;
          if (c) {
            for (a = 0; a < d; a += 1) {
              f = s(t[a], a);
              if (((u = h.get(f).e).f & $) === 0) {
                u.nodes?.a?.measure();
                (o ??= new Set()).add(u);
              }
            }
          }
          let l;
          for (a = 0; a < d; a += 1) {
            f = s(t[a], a);
            u = h.get(f).e;
            if (e.outrogroups !== null) {
              for (const t of e.outrogroups) {
                t.pending.delete(u);
                t.done.delete(u);
              }
            }
            if ((u.f & k) !== 0) {
              kt(u);
              if (c) {
                u.nodes?.a?.unfix();
                (o ??= new Set()).delete(u);
              }
            }
            if ((u.f & $) !== 0) {
              u.f ^= $;
              if (u !== v) {
                const m = p ? p.next : v;
                if (u === e.effect.last) {
                  e.effect.last = u.prev;
                }
                if (u.prev) {
                  u.prev.next = u.next;
                }
                if (u.next) {
                  u.next.prev = u.prev;
                }
                $n(e, p, u);
                $n(e, u, m);
                En(u, m, n);
                _ = [];
                g = [];
                v = Mn((p = u).next);
                continue;
              }
              En(u, null, n);
            }
            if (u !== v) {
              if (l !== undefined && l.has(u)) {
                if (_.length < g.length) {
                  var y;
                  const b = g[0];
                  p = b.prev;
                  const w = _[0];
                  const x = _[_.length - 1];
                  for (y = 0; y < _.length; y += 1) {
                    En(_[y], b, n);
                  }
                  for (y = 0; y < g.length; y += 1) {
                    l.delete(g[y]);
                  }
                  $n(e, w.prev, x.next);
                  $n(e, p, w);
                  $n(e, x, b);
                  v = b;
                  p = x;
                  a -= 1;
                  _ = [];
                  g = [];
                } else {
                  l.delete(u);
                  En(u, v, n);
                  $n(e, u.prev, u.next);
                  $n(e, u, p === null ? e.effect.first : p.next);
                  $n(e, p, u);
                  p = u;
                }
                continue;
              }
              _ = [];
              g = [];
              while (v !== null && v !== u) {
                (l ??= new Set()).add(v);
                g.push(v);
                v = Mn(v.next);
              }
              if (v === null) {
                continue;
              }
            }
            if ((u.f & $) === 0) {
              _.push(u);
            }
            p = u;
            v = Mn(u.next);
          }
          if (e.outrogroups !== null) {
            for (const t of e.outrogroups) {
              if (t.pending.size === 0) {
                xn(e, i(t.done));
                e.outrogroups?.delete(t);
              }
            }
            if (e.outrogroups.size === 0) {
              e.outrogroups = null;
            }
          }
          if (v !== null || l !== undefined) {
            const S = [];
            if (l !== undefined) {
              for (u of l) {
                if ((u.f & k) === 0) {
                  S.push(u);
                }
              }
            }
            while (v !== null) {
              if ((v.f & k) === 0 && v !== e.fallback) {
                S.push(v);
              }
              v = Mn(v.next);
            }
            const j = S.length;
            if (j > 0) {
              const M = r & 4 && d === 0 ? n : null;
              if (c) {
                for (a = 0; a < j; a += 1) {
                  S[a].nodes?.a?.measure();
                }
                for (a = 0; a < j; a += 1) {
                  S[a].nodes?.a?.fix();
                }
              }
              (function (e, t, n) {
                let r;
                for (var s = t.length, l = t.length, o = 0; o < s; o++) {
                  const n = t[o];
                  bt(n, () => {
                    if (r) {
                      r.pending.delete(n);
                      r.done.add(n);
                      if (r.pending.size === 0) {
                        const t = e.outrogroups;
                        xn(e, i(r.done));
                        t.delete(r);
                        if (t.size === 0) {
                          e.outrogroups = null;
                        }
                      }
                    } else {
                      l -= 1;
                    }
                  }, false);
                }
                if (l === 0) {
                  const f = n !== null && e.pending.size === 0;
                  if (f) {
                    const u = n;
                    const a = u.parentNode;
                    a.textContent = "";
                    a.append(u);
                    e.items.clear();
                  }
                  xn(e, t, !f);
                } else {
                  r = {
                    pending: new Set(t),
                    done: new Set()
                  };
                  (e.outrogroups ??= new Set()).add(r);
                }
              })(e, S, M);
            }
          }
          if (c) {
            Y(() => {
              if (o !== undefined) {
                for (u of o) {
                  u.nodes?.a?.apply();
                }
              }
            });
          }
        })(_, t, f, n, s);
        if (a !== null) {
          if (t.length === 0) {
            if ((a.f & $) === 0) {
              kt(a);
            } else {
              a.f ^= $;
              En(a, null, f);
            }
          } else {
            bt(a, () => {
              a = null;
            });
          }
        }
      }
    }
    const p = vt(() => {
      const e = Ut(c);
      for (var t = e.length, i = new Set(), p = ye, _ = 0; _ < t; _ += 1) {
        const g = e[_];
        const m = s(g, _);
        let y = h ? null : u.get(m);
        if (y) {
          if (y.v) {
            Ue(y.v, g);
          }
          if (y.i) {
            Ue(y.i, _);
          }
        } else {
          y = Pn(u, h ? f : Sn ??= et(), g, m, _, l, n, r);
          if (!h) {
            y.e.f |= $;
          }
          u.set(m, y);
        }
        i.add(m);
      }
      if (t === 0 && o && !a) {
        if (h) {
          a = pt(() => o(f));
        } else {
          (a = pt(() => o(Sn ??= et()))).f |= $;
        }
      }
      if (t > i.size) {
        (function () {
          throw new Error("https://svelte.dev/e/each_key_duplicate");
        })();
      }
      if (!h) {
        d.set(p, i);
        v(p);
      }
      Ut(c);
    });
    var _ = {
      effect: p,
      items: u,
      pending: d,
      outrogroups: null,
      fallback: a
    };
    h = false;
  }
  function Mn(e) {
    while (e !== null && (e.f & _) === 0) {
      e = e.next;
    }
    return e;
  }
  function Pn(e, t, n, r, i, s, l, o) {
    const f = l & 1 ? l & 16 ? He(n) : function (e, t = false) {
      const n = He(e);
      if (!t) {
        n.equals = W;
      }
      return n;
    }(n, false, false) : null;
    const u = l & 2 ? He(i) : null;
    return {
      v: f,
      i: u,
      e: pt(() => {
        s(t, f ?? n, u ?? i, o);
        return () => {
          e.delete(r);
        };
      })
    };
  }
  function En(e, t, n) {
    if (e.nodes) {
      for (let r = e.nodes.start, i = e.nodes.end, s = t && (t.f & $) === 0 ? t.nodes.start : n; r !== null;) {
        const l = nt(r);
        s.before(r);
        if (r === i) {
          return;
        }
        r = l;
      }
    }
  }
  function $n(e, t, n) {
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
  function On(e, t) {
    let n;
    n = () => {
      const n = (e = Ot?.parent?.nodes?.start ?? e).getRootNode();
      const r = n.host ? n : n.head ?? n.ownerDocument.head;
      if (!r.querySelector("#" + t.hash)) {
        const e = ot("style");
        e.id = t.hash;
        e.textContent = t.code;
        r.appendChild(e);
      }
    };
    ut(4, n);
  }
  const qn = [..." \t\n\r\f\xA0\v﻿"];
  function An(e, t, n, r, i, s) {
    const l = e[D];
    if (l !== n || l === undefined) {
      const o = function (e, t, n) {
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
                const o = l + s;
                if (l !== 0 && !qn.includes(r[l - 1]) || o !== r.length && !qn.includes(r[o])) {
                  l = o;
                } else {
                  r = (l === 0 ? "" : r.substring(0, l)) + r.substring(o + 1);
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
      }(n, r, s);
      if (o == null) {
        e.removeAttribute("class");
      } else {
        e.className = o;
      }
      e[D] = n;
    } else if (s && i !== s) {
      for (const f in s) {
        const u = !!s[f];
        if (i == null || u !== !!i[f]) {
          e.classList.toggle(f, u);
        }
      }
    }
    return s;
  }
  function Tn(e) {
    const t = e.type;
    return t === "number" || t === "range";
  }
  function zn(e) {
    if (e === "") {
      return null;
    } else {
      return +e;
    }
  }
  const Cn = {
    get(e, t) {
      let n = e.props.length;
      while (n--) {
        let r = e.props[n];
        if (c(r)) {
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
        if (c(i)) {
          i = i();
        }
        const s = l(i, t);
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
        if (c(r)) {
          r = r();
        }
        if (typeof r == "object" && r !== null && t in r) {
          const e = l(r, t);
          if (e && !e.configurable) {
            e.configurable = true;
          }
          return e;
        }
      }
    },
    has(e, t) {
      if (t === A || t === z) {
        return false;
      }
      for (let n of e.props) {
        if (c(n)) {
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
        if (c(n)) {
          n = n();
        }
        if (n) {
          for (const e in n) {
            if (!t.includes(e)) {
              t.push(e);
            }
          }
          for (const e of Object.getOwnPropertySymbols(n)) {
            if (!t.includes(e)) {
              t.push(e);
            }
          }
        }
      }
      return t;
    }
  };
  if (typeof window != "undefined") {
    ((window.__svelte ??= {}).v ??= new Set()).add("5");
  }
  var Dn = vn("<span> <!> <button>pick</button></span>");
  function Nn(e, t) {
    G(t, true);
    const n = function (e, t, n, r) {
      let i = r;
      let s = true;
      const l = () => {
        if (s) {
          s = false;
          i = r;
        }
        return i;
      };
      if (e[t] === undefined && r !== undefined) {
        l();
      }
      return () => {
        const n = e[t];
        if (n === undefined) {
          return l();
        } else {
          s = true;
          return n;
        }
      };
    }(t, "tone", 0, "info");
    const r = Dn();
    const i = rt(r);
    const s = lt(i);
    (function (e, t, ...n) {
      const r = new wn(e);
      vt(() => {
        const e = t() ?? null;
        r.ensure(e, e && ((t) => e(t, ...n)));
      }, M);
    })(s, () => t.children ?? d);
    const l = lt(s, 2);
    ht(() => {
      An(r, 0, `badge ${n() ?? ""}`);
      mn(i, `${t.label ?? ""} `);
    });
    ln("click", l, () => t.onpick?.(t.label));
    pn(e, r);
    J();
  }
  on(["click"]);
  const Ln = vn("<!> <!> <p> </p> <button class=\"more\">more</button>", 1);
  on(["click"]);
  const Rn = vn("<button> </button> <p> </p>", 1);
  on(["click"]);
  const Fn = vn("<p> </p>");
  const Hn = vn("<span> </span>");
  const Kn = vn("<!> <!> <button>flip</button>", 1);
  on(["click"]);
  const Vn = vn("<p class=\"err\"> </p>");
  const Wn = vn("<p> </p>");
  const Bn = vn("<!> <button>break</button>", 1);
  on(["click"]);
  const In = vn("<!> <button class=\"swap\">swap</button>", 1);
  on(["click"]);
  const Gn = vn("<li><input type=\"checkbox\"/> <button class=\"remove\">x</button></li>");
  const Jn = vn("<input placeholder=\"new todo\"/> <button class=\"add\">add</button> <ul></ul> <p> </p>", 1);
  const Qn = {
    hash: "svelte-70rljk",
    code: ".done.svelte-70rljk {text-decoration:line-through;}"
  };
  on(["click"]);
  let We;
  window.__mount = function (e, t) {
    return function (e, { target: t, anchor: n, props: r = {}, events: s, context: o, intro: f = true, transformError: u }) {
      (function () {
        if (We === undefined) {
          We = window;
          Be = /Firefox/.test(navigator.userAgent);
          const e = Element.prototype;
          const t = Node.prototype;
          const n = Text.prototype;
          Ie = l(t, "firstChild").get;
          Ge = l(t, "nextSibling").get;
          if (a(e)) {
            e[D] = undefined;
            e[C] = null;
            e[N] = undefined;
            e.__e = undefined;
          }
          if (a(n)) {
            n[L] = undefined;
          }
        }
      })();
      let c = undefined;
      const d = function (e) {
        $e.ensure();
        const t = ut(524352, e);
        return (e = {}) => new Promise((n) => {
          if (e.outro) {
            bt(t, () => {
              mt(t);
              n(undefined);
            });
          } else {
            mt(t);
            n(undefined);
          }
        });
      }(() => {
        const l = n ?? t.appendChild(et());
        _n(l, { pending: () => {} }, (t) => {
          G({});
          if (o) {
            B.c = o;
          }
          if (s) {
            r.$$events = s;
          }
          c = e(t, r) || Q();
          J();
        }, u);
        const f = new Set();
        const a = (e) => {
          for (let n = 0; n < e.length; n++) {
            const r = e[n];
            if (!f.has(r)) {
              f.add(r);
              const i = tn(r);
              for (const e of [t, document]) {
                let s = yn.get(e);
                if (s === undefined) {
                  s = new Map();
                  yn.set(e, s);
                }
                const l = s.get(r);
                if (l === undefined) {
                  e.addEventListener(r, an, { passive: i });
                  s.set(r, 1);
                } else {
                  s.set(r, l + 1);
                }
              }
            }
          }
        };
        a(i(rn));
        sn.add(a);
        return () => {
          for (const e of f) {
            for (const n of [t, document]) {
              const r = yn.get(n);
              let i = r.get(e);
              if (--i == 0) {
                n.removeEventListener(e, an);
                r.delete(e);
                if (r.size === 0) {
                  yn.delete(n);
                }
              } else {
                r.set(e, i);
              }
            }
          }
          sn.delete(a);
          if (l !== n) {
            l.parentNode?.removeChild(l);
          }
        };
      });
      bn.set(c, d);
      return c;
    }(e, t);
  };
  window.__component = [
    function (e) {
      const t = Ke("none");
      const n = Ke(1);
      const r = Ln();
      const i = it(r);
      Nn(i, {
        label: "first",
        onpick: (e) => Ve(t, e, true),
        children: (e, t) => {
          const r = function (e = "") {
            const t = et(e + "");
            hn(t, t);
            return t;
          }();
          ht(() => mn(r, `inner ${Ut(n) ?? ""}`));
          pn(e, r);
        },
        $$slots: { default: true }
      });
      const s = lt(i, 2);
      Nn(s, {
        label: "second",
        tone: "warn"
      });
      const l = lt(s, 2);
      const o = st(l);
      const f = lt(l, 2);
      ht(() => mn(o, `picked ${Ut(t) ?? ""}`));
      ln("click", f, () => function (e, t = 1) {
        let n = Ut(e);
        const r = t === 1 ? n++ : n--;
        Ve(e, n);
        return r;
      }(n));
      pn(e, r);
    },
    function (e) {
      const t = Ke(0);
      const n = de(() => Ut(t) * 2);
      const r = Rn();
      const i = it(r);
      const s = st(i);
      const l = st(lt(i, 2));
      ht(() => {
        mn(s, `clicks: ${Ut(t) ?? ""}`);
        mn(l, `doubled is ${Ut(n) ?? ""}`);
      });
      ln("click", i, function () {
        Ve(t, Ut(t) + 1);
      });
      pn(e, r);
    },
    function (e) {
      const t = Ze([{
        id: 1,
        pos: { x: 3 }
      }, {
        id: 2,
        pos: {}
      }]);
      const n = Kn();
      const r = it(n);
      jn(r, 19, () => t, ({ id: e, pos: { x: t = 9 } }) => e, (e, t, n) => {
        const r = he(() => v(Ut(t).pos.x, 9));
        const i = Fn();
        const s = st(i);
        ht(() => mn(s, `${Ut(n) ?? ""}: ${Ut(t).id ?? ""} at ${Ut(r) ?? ""}`));
        pn(e, i);
      });
      const i = lt(r, 2);
      jn(i, 16, () => [[1, 2], [3]], kn, (e, t) => {
        const n = de(() => function (e, t) {
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
        }(t, 2));
        const r = he(() => v(Ut(n)[1], 0));
        const i = Hn();
        const s = st(i);
        ht(() => mn(s, `${Ut(n)[0] ?? ""}-${Ut(r) ?? ""}`));
        pn(e, i);
      });
      ln("click", lt(i, 2), () => t.reverse());
      pn(e, n);
    },
    function (e, t) {
      G(t, true);
      const n = Ke(false);
      const r = Bn();
      const i = it(r);
      {
        const e = (e, t = d, n = d) => {
          const r = Vn();
          const i = st(r, true);
          ht(() => mn(i, t().message));
          pn(e, r);
        };
        _n(i, { failed: e }, (e) => {
          const t = Wn();
          const r = st(t, true);
          ht((e) => mn(r, e), [() => function () {
            if (Ut(n)) {
              throw new Error("boom");
            }
            return "fine";
          }()]);
          pn(e, t);
        });
      }
      ln("click", lt(i, 2), () => Ve(n, true));
      pn(e, r);
      J();
    },
    function (e) {
      const t = Ke(Ze({
        label: "spread",
        tone: "warn"
      }));
      const n = In();
      const r = it(n);
      Nn(r, function (...e) {
        return new Proxy({ props: e }, Cn);
      }(() => Ut(t)));
      ln("click", lt(r, 2), () => Ve(t, { label: "other" }, true));
      pn(e, n);
    },
    function (e, t) {
      G(t, true);
      On(e, Qn);
      const n = Ke(Ze([{
        id: 1,
        text: "write",
        done: false
      }]));
      const r = Ke("");
      let i = 2;
      const s = de(() => Ut(n).filter((e) => !e.done).length);
      const l = Jn();
      const o = it(l);
      const f = lt(o, 2);
      const u = lt(f, 2);
      jn(u, 21, () => Ut(n), (e) => e.id, (e, t, r) => {
        const i = Gn();
        let s;
        const l = rt(i);
        const o = lt(l);
        const f = lt(o);
        ht(() => {
          s = An(i, 0, "svelte-70rljk", null, s, { done: Ut(t).done });
          mn(o, ` ${Ut(t).text ?? ""} `);
        });
        (function (e, t, n = t) {
          le(e, "change", (t) => {
            const r = t ? e.defaultChecked : e.checked;
            n(r);
          });
          if (Zt(t) == null) {
            n(e.checked);
          }
          dt(() => {
            const n = t();
            e.checked = Boolean(n);
          });
        })(l, () => Ut(t).done, (e) => Ut(t).done = e);
        ln("click", f, () => {
          const e = Ut(t).id;
          Ve(n, Ut(n).filter((t) => t.id !== e), true);
          return;
        });
        pn(e, i);
      });
      const a = st(lt(u, 2));
      ht(() => mn(a, `${Ut(s) ?? ""} left`));
      (function (e, t, n = t) {
        const r = new WeakSet();
        le(e, "input", async (i) => {
          let s = i ? e.defaultValue : e.value;
          s = Tn(e) ? zn(s) : s;
          n(s);
          if (ye !== null) {
            r.add(ye);
          }
          await Qt();
          if (s !== (s = t())) {
            const { selectionStart: l, selectionEnd: o } = e;
            const f = e.value.length;
            e.value = s ?? "";
            if (o !== null) {
              const u = e.value.length;
              if (l === o && o === f && u > f) {
                e.selectionStart = u;
                e.selectionEnd = u;
              } else {
                e.selectionStart = l;
                e.selectionEnd = Math.min(o, u);
              }
            }
          }
        });
        if (Zt(t) == null && e.value) {
          n(Tn(e) ? zn(e.value) : e.value);
          if (ye !== null) {
            r.add(ye);
          }
        }
        dt(() => {
          const n = t();
          if (e === document.activeElement) {
            const i = ye;
            if (r.has(i)) {
              return;
            }
          }
          if (!Tn(e) || n !== zn(e.value)) {
            if ((e.type !== "date" || n || e.value) && n !== e.value) {
              e.value = n ?? "";
            }
          }
        });
      })(o, () => Ut(r), (e) => Ve(r, e));
      ln("click", f, function () {
        if (Ut(r)) {
          Ut(n).push({
            id: i++,
            text: Ut(r),
            done: false
          });
          Ve(r, "");
        }
      });
      pn(e, l);
      J();
    }
  ];
})();