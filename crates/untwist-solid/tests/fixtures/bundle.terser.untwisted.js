(function () {
  "use strict";
  const e = Symbol("solid-track");
  const t = { equals: (e, t) => e === t };
  const n = $;
  const r = 1;
  const l = 2;
  const o = {
    owned: null,
    cleanups: null,
    context: null,
    owner: null
  };
  var s = null;
  const i = null;
  let u = null;
  let c = null;
  let a = null;
  let f = 0;
  function d(e, t) {
    const n = u;
    const r = s;
    const l = e.length === 0;
    const i = t === undefined ? r : t;
    const c = l ? o : {
      owned: null,
      cleanups: null,
      context: i ? i.context : null,
      owner: i
    };
    const a = l ? e : () => e(function (e) {
      return () => g(() => N(e));
    }(c));
    s = c;
    u = null;
    try {
      return x(a, true);
    } finally {
      u = n;
      s = r;
    }
  }
  function h(e, n) {
    const r = {
      value: e,
      observers: null,
      observerSlots: null,
      comparator: (n = n ? Object.assign({}, t, n) : t).equals || undefined
    };
    return [w.bind(r), (e) => {
      if (typeof e == "function") {
        e = e(r.value);
      }
      return A(r, e);
    }];
  }
  function p(e, t, n) {
    m(S(e, t, false, r));
  }
  function v(e, n, r) {
    r = r ? Object.assign({}, t, r) : t;
    const l = S(e, n, true, 0);
    l.observers = null;
    l.observerSlots = null;
    l.comparator = r.equals || undefined;
    m(l);
    return w.bind(l);
  }
  function g(e) {
    if (u === null) {
      return e();
    }
    const t = u;
    u = null;
    try {
      return e();
    } finally {
      u = t;
    }
  }
  function b(e) {
    const t = v(e);
    const n = v(() => E(t()));
    n.toArray = () => {
      const e = n();
      if (Array.isArray(e)) {
        return e;
      } else if (e != null) {
        return [e];
      } else {
        return [];
      }
    };
    return n;
  }
  function y(e) {
    e.observerSlots = e.observers.map((t, n) => {
      const r = t.sources;
      if (!r) {
        return -1;
      }
      for (let l = r.length - 1; l >= 0; l--) {
        if (r[l] === e && t.sourceSlots && t.sourceSlots[l] === n) {
          return l;
        }
      }
      return r.lastIndexOf(e);
    });
  }
  function w() {
    if (this.sources && this.state) {
      if (this.state === r) {
        m(this);
      } else {
        const e = c;
        c = null;
        x(() => C(this), false);
        c = e;
      }
    }
    if (u) {
      const e = this.observers;
      if (!e || e[e.length - 1] !== u) {
        const t = e ? e.length : 0;
        if (u.sources) {
          u.sources.push(this);
          u.sourceSlots.push(t);
        } else {
          const e = [this];
          const n = [t];
          u.sources = e;
          u.sourceSlots = n;
        }
        if (e) {
          if (!this.observerSlots || this.observerSlots.length !== e.length) {
            y(this);
          }
          e.push(u);
          this.observerSlots.push(u.sources.length - 1);
        } else {
          const e = [u];
          const t = [u.sources.length - 1];
          this.observers = e;
          this.observerSlots = t;
        }
      }
    }
    return this.value;
  }
  function A(e, t, n) {
    const l = e.value;
    if (!e.comparator || !e.comparator(l, t)) {
      e.value = t;
      if (e.observers && e.observers.length) {
        x(() => {
          for (let t = 0; t < e.observers.length; t += 1) {
            const n = e.observers[t];
            const l = i && i.running;
            if (l) {
              i.disposed.has(n);
            }
            if (!(l ? n.tState : n.state)) {
              if (n.pure) {
                c.push(n);
              } else {
                a.push(n);
              }
              if (n.observers) {
                T(n);
              }
            }
            if (!l) {
              n.state = r;
            }
          }
          if (c.length > 1e6) {
            c = [];
            throw new Error();
          }
        }, false);
      }
    }
    return t;
  }
  function m(e) {
    if (!e.fn) {
      return;
    }
    N(e, true);
    const t = f;
    (function (e, t, n) {
      let l;
      const o = s;
      const i = u;
      u = s = e;
      try {
        l = e.fn(t);
      } catch (t) {
        if (e.pure) {
          e.state = r;
          if (e.owned) {
            e.owned.forEach((e) => N(e));
          }
          e.owned = null;
        }
        e.updatedAt = n + 1;
        return _(t);
      } finally {
        u = i;
        s = o;
      }
      if (!e.updatedAt || e.updatedAt <= n) {
        if (e.updatedAt != null && "observers" in e) {
          A(e, l);
        } else {
          e.value = l;
        }
        e.updatedAt = n;
      }
    })(e, e.value, t);
  }
  function S(e, t, n, l = r, i) {
    const u = {
      fn: e,
      state: l,
      updatedAt: null,
      owned: null,
      sources: null,
      sourceSlots: null,
      cleanups: null,
      value: t,
      owner: s,
      context: s ? s.context : null,
      pure: n
    };
    if (s !== null) {
      if (s !== o) {
        if (s.owned) {
          s.owned.push(u);
        } else {
          s.owned = [u];
        }
      }
    }
    return u;
  }
  function k(e) {
    if (e.state === 0) {
      return;
    }
    if (e.state === l) {
      return C(e);
    }
    if (e.suspense && g(e.suspense.inFallback)) {
      return e.suspense.effects.push(e);
    }
    const t = [e];
    while ((e = e.owner) && (!e.updatedAt || e.updatedAt < f)) {
      if (e.state) {
        t.push(e);
      }
    }
    for (let n = t.length - 1; n >= 0; n--) {
      if ((e = t[n]).state === r) {
        m(e);
      } else if (e.state === l) {
        const n = c;
        c = null;
        x(() => C(e, t[0]), false);
        c = n;
      }
    }
  }
  function x(e, t) {
    if (c) {
      return e();
    }
    let r = false;
    if (!t) {
      c = [];
    }
    if (a) {
      r = true;
    } else {
      a = [];
    }
    f++;
    try {
      const t = e();
      (function (e) {
        if (c) {
          $(c);
          c = null;
        }
        if (e) {
          return;
        }
        const t = a;
        a = null;
        if (t.length) {
          x(() => n(t), false);
        }
      })(r);
      return t;
    } catch (e) {
      if (!r) {
        a = null;
      }
      c = null;
      _(e);
    }
  }
  function $(e) {
    for (let t = 0; t < e.length; t++) {
      k(e[t]);
    }
  }
  function C(e, t) {
    e.state = 0;
    for (let n = 0; n < e.sources.length; n += 1) {
      const o = e.sources[n];
      if (o.sources) {
        const e = o.state;
        if (e === r) {
          if (o !== t && (!o.updatedAt || o.updatedAt < f)) {
            k(o);
          }
        } else if (e === l) {
          C(o, t);
        }
      }
    }
  }
  function T(e) {
    for (let t = 0; t < e.observers.length; t += 1) {
      const n = e.observers[t];
      if (!n.state) {
        n.state = l;
        if (n.pure) {
          c.push(n);
        } else {
          a.push(n);
        }
        if (n.observers) {
          T(n);
        }
      }
    }
  }
  function N(e, t) {
    let n;
    if (e.sources) {
      while (e.sources.length) {
        const n = e.sources.pop();
        const r = e.sourceSlots.pop();
        const l = n.observers;
        if (l && l.length) {
          if (!n.observerSlots || n.observerSlots.length !== l.length) {
            y(n);
          }
          const e = l.pop();
          const t = n.observerSlots.pop();
          if (r < l.length) {
            e.sourceSlots[t] = r;
            l[r] = e;
            n.observerSlots[r] = t;
          }
        }
        if (!t) {
          if (l) {
            l.length;
          }
        }
      }
    }
    if (e.tOwned) {
      for (n = e.tOwned.length - 1; n >= 0; n--) {
        N(e.tOwned[n]);
      }
      delete e.tOwned;
    }
    if (e.owned) {
      for (n = e.owned.length - 1; n >= 0; n--) {
        N(e.owned[n]);
      }
      e.owned = null;
    }
    if (e.cleanups) {
      for (n = e.cleanups.length - 1; n >= 0; n--) {
        e.cleanups[n]();
      }
      e.cleanups = null;
    }
    e.state = 0;
  }
  function _(e, t = s) {
    const n = function (e) {
      if (e instanceof Error) {
        return e;
      } else {
        return new Error(typeof e == "string" ? e : "Unknown error", { cause: e });
      }
    }(e);
    throw n;
  }
  function E(e) {
    if (typeof e == "function" && !e.length) {
      return E(e());
    }
    if (Array.isArray(e)) {
      const t = [];
      for (let n = 0; n < e.length; n++) {
        const r = E(e[n]);
        if (Array.isArray(r)) {
          if (r.length < 32768) {
            t.push.apply(t, r);
          } else {
            for (let e = 0; e < r.length; e++) {
              t.push(r[e]);
            }
          }
        } else {
          t.push(r);
        }
      }
      return t;
    }
    return e;
  }
  function O(e, t) {
    return function (t) {
      let n;
      p(() => n = g(() => {
        s.context = {
          ...s.context,
          [e]: t.value
        };
        return b(() => t.children);
      }), undefined);
      return n;
    };
  }
  const B = Symbol("fallback");
  function P(e) {
    for (let t = 0; t < e.length; t++) {
      e[t]();
    }
  }
  function j(t, n, r = {}) {
    let l = [];
    let o = [];
    let i = [];
    let u = 0;
    let c = n.length > 1 ? [] : null;
    let a;
    a = () => P(i);
    if (s !== null) {
      if (s.cleanups === null) {
        s.cleanups = [a];
      } else {
        s.cleanups.push(a);
      }
    }
    return () => {
      let s;
      let a;
      const f = t() || [];
      const p = f.length;
      f[e];
      return g(() => {
        let e;
        let t;
        let n;
        let h;
        let g;
        let b;
        let y;
        let w;
        let A;
        if (p === 0) {
          if (u !== 0) {
            P(i);
            i = [];
            l = [];
            o = [];
            u = 0;
            c &&= [];
          }
          if (r.fallback) {
            l = [B];
            o[0] = d((e) => {
              i[0] = e;
              return r.fallback();
            });
            u = 1;
          }
        } else if (u === 0) {
          o = new Array(p);
          a = 0;
          for (; a < p; a++) {
            l[a] = f[a];
            o[a] = d(v);
          }
          u = p;
        } else {
          n = new Array(p);
          h = new Array(p);
          if (c) {
            g = new Array(p);
          }
          b = 0;
          y = Math.min(u, p);
          for (; b < y && l[b] === f[b]; b++);
          y = u - 1;
          w = p - 1;
          for (; y >= b && w >= b && l[y] === f[w]; y--, w--) {
            n[w] = o[y];
            h[w] = i[y];
            if (c) {
              g[w] = c[y];
            }
          }
          e = new Map();
          t = new Array(w + 1);
          a = w;
          for (; a >= b; a--) {
            A = f[a];
            s = e.get(A);
            t[a] = s === undefined ? -1 : s;
            e.set(A, a);
          }
          for (s = b; s <= y; s++) {
            A = l[s];
            a = e.get(A);
            if (a !== undefined && a !== -1) {
              n[a] = o[s];
              h[a] = i[s];
              if (c) {
                g[a] = c[s];
              }
              a = t[a];
              e.set(A, a);
            } else {
              i[s]();
            }
          }
          for (a = b; a < p; a++) {
            if (a in n) {
              o[a] = n[a];
              i[a] = h[a];
              if (c) {
                c[a] = g[a];
                c[a](a);
              }
            } else {
              o[a] = d(v);
            }
          }
          o = o.slice(0, u = p);
          l = f.slice(0);
        }
        return o;
      });
      function v(e) {
        i[a] = e;
        if (c) {
          const [e, t] = h(a);
          c[a] = t;
          return n(f[a], e);
        }
        return n(f[a]);
      }
    };
  }
  function q(e, t) {
    return g(() => e(t || {}));
  }
  function L(e) {
    const t = "fallback" in e && { fallback: () => e.fallback };
    return v(j(() => e.each, e.children, t || undefined));
  }
  function M(e) {
    const t = e.keyed;
    const n = v(() => e.when, undefined, undefined);
    const r = t ? n : v(n, undefined, { equals: (e, t) => !e == !t });
    return v(() => {
      const l = r();
      if (l) {
        const o = e.children;
        if (typeof o == "function" && o.length > 0) {
          return g(() => o(t ? l : () => {
            if (!g(r)) {
              throw `Stale read from <Show>.`;
            }
            return n();
          }));
        } else {
          return o;
        }
      }
      return e.fallback;
    }, undefined, undefined);
  }
  const D = (e) => v(() => e());
  const V = "_$DX_DELEGATE";
  function z(e, t, n, r) {
    let l;
    const o = () => (l ||= (() => {
      const t = document.createElement("template");
      t.innerHTML = e;
      return t.content.firstChild;
    })()).cloneNode(true);
    o.cloneNode = o;
    return o;
  }
  function F(e, t = window.document) {
    const n = t[V] ||= new Set();
    for (let r = 0, l = e.length; r < l; r++) {
      const l = e[r];
      if (!n.has(l)) {
        n.add(l);
        t.addEventListener(l, H);
      }
    }
  }
  function G(e, t, n, r) {
    if (n !== undefined && !r) {
      r = [];
    }
    if (typeof t != "function") {
      return I(e, t, r, n);
    }
    p((r) => I(e, t(), r, n), r);
  }
  function H(e) {
    let t = e.target;
    const n = `$$${e.type}`;
    const r = e.target;
    const l = e.currentTarget;
    const o = (t) => Object.defineProperty(e, "target", {
      configurable: true,
      value: t
    });
    const s = () => {
      const r = t[n];
      if (r && !t.disabled) {
        const l = t[`${n}Data`];
        if (l !== undefined) {
          r.call(t, l, e);
        } else {
          r.call(t, e);
        }
        if (e.cancelBubble) {
          return;
        }
      }
      if (t.host && typeof t.host != "string" && !t.host._$host && t.contains(e.target)) {
        o(t.host);
      }
      return true;
    };
    const i = () => {
      while (s() && (t = t._$host || t.parentNode || t.host));
    };
    Object.defineProperty(e, "currentTarget", {
      configurable: true,
      get: () => t || document
    });
    try {
      if (e.composedPath) {
        const n = e.composedPath();
        o(n[0]);
        for (let e = 0; e < n.length - 2 && (t = n[e], s()); e++) {
          if (t._$host) {
            t = t._$host;
            i();
            break;
          }
          if (t.parentNode === l) {
            break;
          }
        }
      } else {
        i();
      }
      o(r);
    } finally {
      e = t = null;
    }
  }
  function I(e, t, n, r, l) {
    while (typeof n == "function") {
      n = n();
    }
    if (t === n) {
      return n;
    }
    const o = typeof t;
    const s = r !== undefined;
    e = s && n[0] && n[0].parentNode || e;
    if (o === "string" || o === "number") {
      if (o === "number" && (t = t.toString()) === n) {
        return n;
      }
      if (s) {
        let l = n[0];
        if (l && l.nodeType === 3) {
          if (l.data !== t) {
            l.data = t;
          }
        } else {
          l = document.createTextNode(t);
        }
        n = J(e, n, r, l);
      } else {
        n = n !== "" && typeof n == "string" ? e.firstChild.data = t : e.textContent = t;
      }
    } else if (t == null || o === "boolean") {
      n = J(e, n, r);
    } else {
      if (o === "function") {
        p(() => {
          let l = t();
          while (typeof l == "function") {
            l = l();
          }
          n = I(e, l, n, r);
        });
        return () => n;
      }
      if (Array.isArray(t)) {
        const o = [];
        const i = n && Array.isArray(n);
        if (U(o, t, n, l)) {
          p(() => n = I(e, o, n, r, true));
          return () => n;
        }
        if (o.length === 0) {
          n = J(e, n, r);
          if (s) {
            return n;
          }
        } else if (i) {
          if (n.length === 0) {
            X(e, o, r);
          } else {
            (function (e, t, n) {
              const r = n.length;
              let l = t.length;
              let o = r;
              let s = 0;
              let i = 0;
              const u = t[l - 1].nextSibling;
              let c = null;
              while (s < l || i < o) {
                if (t[s] !== n[i]) {
                  while (t[l - 1] === n[o - 1]) {
                    l--;
                    o--;
                  }
                  if (l === s) {
                    const t = o < r ? i ? n[i - 1].nextSibling : n[o - i] : u;
                    while (i < o) {
                      e.insertBefore(n[i++], t);
                    }
                  } else if (o === i) {
                    while (s < l) {
                      if (!c || !c.has(t[s])) {
                        t[s].remove();
                      }
                      s++;
                    }
                  } else if (t[s] === n[o - 1] && n[i] === t[l - 1]) {
                    const r = t[--l].nextSibling;
                    e.insertBefore(n[i++], t[s++].nextSibling);
                    e.insertBefore(n[--o], r);
                    t[l] = n[o];
                  } else {
                    if (!c) {
                      c = new Map();
                      let e = i;
                      while (e < o) {
                        c.set(n[e], e++);
                      }
                    }
                    const r = c.get(t[s]);
                    if (r != null) {
                      if (i < r && r < o) {
                        let u;
                        let a = s;
                        let f = 1;
                        while (++a < l && a < o && (u = c.get(t[a])) != null && u === r + f) {
                          f++;
                        }
                        if (f > r - i) {
                          const l = t[s];
                          while (i < r) {
                            e.insertBefore(n[i++], l);
                          }
                        } else {
                          e.replaceChild(n[i++], t[s++]);
                        }
                      } else {
                        s++;
                      }
                    } else {
                      t[s++].remove();
                    }
                  }
                } else {
                  s++;
                  i++;
                }
              }
            })(e, n, o);
          }
        } else {
          if (n) {
            J(e);
          }
          X(e, o);
        }
        n = o;
      } else if (t.nodeType) {
        if (Array.isArray(n)) {
          if (s) {
            return n = J(e, n, r, t);
          }
          J(e, n, null, t);
        } else if (n != null && n !== "" && e.firstChild) {
          e.replaceChild(t, e.firstChild);
        } else {
          e.appendChild(t);
        }
        n = t;
      }
    }
    return n;
  }
  function U(e, t, n, r) {
    let l = false;
    for (let o = 0, s = t.length; o < s; o++) {
      let s;
      let i = t[o];
      const u = n && n[e.length];
      if (i == null || i === true || i === false);
      else if ((s = typeof i) == "object" && i.nodeType) {
        e.push(i);
      } else if (Array.isArray(i)) {
        l = U(e, i, u) || l;
      } else if (s === "function") {
        if (r) {
          while (typeof i == "function") {
            i = i();
          }
          l = U(e, Array.isArray(i) ? i : [i], Array.isArray(u) ? u : [u]) || l;
        } else {
          e.push(i);
          l = true;
        }
      } else {
        const t = String(i);
        if (u && u.nodeType === 3 && u.data === t) {
          e.push(u);
        } else {
          e.push(document.createTextNode(t));
        }
      }
    }
    return l;
  }
  function X(e, t, n = null) {
    for (let r = 0, l = t.length; r < l; r++) {
      e.insertBefore(t[r], n);
    }
  }
  function J(e, t, n, r) {
    if (n === undefined) {
      return e.textContent = "";
    }
    const l = r || document.createTextNode("");
    if (t.length) {
      let r = false;
      for (let o = t.length - 1; o >= 0; o--) {
        const s = t[o];
        if (l !== s) {
          const t = s.parentNode === e;
          if (r || o) {
            if (t) {
              s.remove();
            }
          } else if (t) {
            e.replaceChild(l, s);
          } else {
            e.insertBefore(l, n);
          }
        } else {
          r = true;
        }
      }
    } else {
      e.insertBefore(l, n);
    }
    return [l];
  }
  var K = z("<span class=badge>");
  function Q(e) {
    let t;
    G(t = K(), () => e.label, null);
    G(t, () => e.children, null);
    p(() => {
      const n = t;
      const r = "data-tone";
      let l;
      if ((l = e.tone ?? "plain") == null) {
        n.removeAttribute(r);
      } else {
        n.setAttribute(r, l);
      }
      return;
    });
    return t;
  }
  const R = z("<em>child ");
  const W = z("<div class=parent><button>more");
  F(["click"]);
  const Y = z("<div><p>Count: </p><button>+1");
  F(["click"]);
  const Z = z("<ul>");
  const ee = z("<p> left");
  const te = z("<div class=todo><input><button class=add>add");
  const ne = z("<p>nothing to do");
  const re = z("<li><button class=rm>x");
  F(["input", "click"]);
  var le = z("<span>");
  const oe = z("<div><button>flip");
  const se = function (e) {
    const t = Symbol("context");
    return {
      id: t,
      Provider: O(t),
      defaultValue: e
    };
  }("light");
  function ie() {
    const e = function (e) {
      let t;
      if (s && s.context && (t = s.context[e.id]) !== undefined) {
        return t;
      } else {
        return e.defaultValue;
      }
    }(se);
    let t;
    G(t = le(), e);
    p(() => {
      const n = t;
      let r;
      if ((r = e()) == null) {
        n.removeAttribute("class");
      } else {
        n.className = r;
      }
      return;
    });
    return t;
  }
  F(["click"]);
  const ue = z("<button>more");
  const ce = z("<div class=mixed><h3>");
  const ae = z("<span class=tag>");
  const fe = z("<s>hidden");
  F(["click"]);
  const de = z("<div><button class=t>toggle</button><button class=n>inc");
  const he = z("<b>on");
  const pe = z("<i>off");
  const ve = z("<span>more than one");
  F(["click"]);
  window.__render = function (e, t, n, r = {}) {
    let l;
    d((r) => {
      l = r;
      if (t === document) {
        e();
      } else {
        G(t, e(), !n && t.firstChild ? null : undefined, n);
      }
    }, r.owner);
    return () => {
      l();
      t.textContent = "";
    };
  };
  window.__component = [
    function () {
      const [e, t] = h(0);
      const n = W();
      const r = n.firstChild;
      G(n, q(Q, { label: "static" }), r);
      G(n, q(Q, {
        get label() {
          return `count ${e()}`;
        },
        get tone() {
          if (e() > 1) {
            return "hot";
          } else {
            return undefined;
          }
        },
        get children() {
          const t = R();
          t.firstChild;
          G(t, e, null);
          return t;
        }
      }), r);
      G(n, q(Q, {
        label: "text",
        children: "plain text"
      }), r);
      r.$$click = () => t(e() + 1);
      return n;
    },
    function () {
      const [e, t] = h(0);
      return (() => {
        const n = Y();
        const r = n.firstChild;
        r.firstChild;
        const l = r.nextSibling;
        G(r, e, null);
        l.$$click = () => t(e() + 1);
        return n;
      })();
    },
    function () {
      const [e, t] = h([]);
      const [n, r] = h("");
      let l = 0;
      const o = () => {
        if (n()) {
          t([...e(), {
            id: l++,
            text: n()
          }]);
          r("");
        }
      };
      const s = te();
      const i = s.firstChild;
      const u = i.nextSibling;
      i.$$input = (e) => r(e.currentTarget.value);
      u.$$click = o;
      G(s, q(M, {
        get when() {
          return e().length;
        },
        get fallback() {
          return ne();
        },
        get children() {
          let n;
          let r;
          return [(r = Z(), G(r, q(L, {
            get each() {
              return e();
            },
            children: (n) => {
              const r = re();
              const l = r.firstChild;
              G(r, () => n.text, l);
              l.$$click = () => t(e().filter((e) => e.id !== n.id));
              return r;
            }
          })), r), (n = ee(), G(n, () => e().length, n.firstChild), n)];
        }
      }), null);
      p(() => i.value = n());
      return s;
    },
    function () {
      const [e, t] = h("dark");
      const n = oe();
      const r = n.firstChild;
      G(n, q(se.Provider, {
        value: e,
        get children() {
          return q(ie, {});
        }
      }), r);
      r.$$click = () => t(e() === "dark" ? "light" : "dark");
      return n;
    },
    function (e) {
      const [t, n] = h(true);
      const [r, l] = h(["x"]);
      const o = ce();
      let s;
      (s = o.firstChild).$$click = () => n(!t());
      G(s, () => e.heading);
      G(o, q(M, {
        get when() {
          return t();
        },
        get children() {
          let e;
          return [q(L, {
            get each() {
              return r();
            },
            children: (e) => q(M, {
              when: e !== "hide",
              get fallback() {
                return fe();
              },
              get children() {
                const t = ae();
                G(t, e);
                return t;
              }
            })
          }), (e = ue(), e.$$click = () => l([...r(), "hide", "y"]), e)];
        }
      }), null);
      p(() => o.classList.toggle("open", !!t()));
      return o;
    },
    function () {
      const [e, t] = h(false);
      const [n, r] = h(0);
      const s = de();
      const i = s.firstChild;
      const u = i.nextSibling;
      let l;
      G(s, (l = D(() => !!e()), () => l() ? he() : pe()), i);
      let o;
      G(s, (o = D(() => n() > 1), () => o() && ve()), i);
      G(s, () => n() > 0 ? "positive" : "zero", i);
      i.$$click = () => t(!e());
      u.$$click = () => r(n() + 1);
      return s;
    }
  ];
})();