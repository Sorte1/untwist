const G = (e, s) => e === s;
const I = { equals: G };
let P = q;
const d = 1;
const x = 2;
const L = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var h = null;
let A = null;
let H = null;
let f = null;
let u = null;
let w = null;
let v = 0;
function Q(e, s) {
  const t = f;
  const n = h;
  const i = e.length === 0;
  const o = s === undefined ? n : s;
  const r = i ? L : {
    owned: null,
    cleanups: null,
    context: o ? o.context : null,
    owner: o
  };
  const l = i ? e : () => e(W(r));
  h = r;
  f = null;
  try {
    return b(l, true);
  } finally {
    f = t;
    h = n;
  }
}
function W(e) {
  return () => N(() => y(e));
}
function Y(e, s) {
  s = s ? Object.assign({}, I, s) : I;
  const t = {
    value: e,
    observers: null,
    observerSlots: null,
    comparator: s.equals || undefined
  };
  const n = (i) => {
    if (typeof i == "function") {
      i = i(t.value);
    }
    return _(t, i);
  };
  return [J.bind(t), n];
}
function T(e, s, t) {
  const n = X(e, s, false, d);
  B(n);
}
function N(e) {
  if (f === null) {
    return e();
  }
  const s = f;
  f = null;
  try {
    return e();
  } finally {
    f = s;
  }
}
function M(e) {
  e.observerSlots = e.observers.map((s, t) => {
    const n = s.sources;
    if (!n) {
      return -1;
    }
    for (let i = n.length - 1; i >= 0; i--) {
      if (n[i] === e && s.sourceSlots && s.sourceSlots[i] === t) {
        return i;
      }
    }
    return n.lastIndexOf(e);
  });
}
function J() {
  if (this.sources && this.state) {
    if (this.state === d) {
      B(this);
    } else {
      const e = u;
      u = null;
      b(() => C(this), false);
      u = e;
    }
  }
  if (f) {
    const e = this.observers;
    if (!e || e[e.length - 1] !== f) {
      const s = e ? e.length : 0;
      if (f.sources) {
        f.sources.push(this);
        f.sourceSlots.push(s);
      } else {
        const t = [this];
        const n = [s];
        f.sources = t;
        f.sourceSlots = n;
      }
      if (e) {
        if (!this.observerSlots || this.observerSlots.length !== e.length) {
          M(this);
        }
        e.push(f);
        this.observerSlots.push(f.sources.length - 1);
      } else {
        const t = [f];
        const n = [f.sources.length - 1];
        this.observers = t;
        this.observerSlots = n;
      }
    }
  }
  return this.value;
}
function _(e, s, t) {
  const n = e.value;
  if (!e.comparator || !e.comparator(n, s)) {
    e.value = s;
    if (e.observers && e.observers.length) {
      b(() => {
        for (let i = 0; i < e.observers.length; i += 1) {
          const o = e.observers[i];
          const r = A && A.running;
          if (r) {
            A.disposed.has(o);
          }
          if (r ? !o.tState : !o.state) {
            if (o.pure) {
              u.push(o);
            } else {
              w.push(o);
            }
            if (o.observers) {
              F(o);
            }
          }
          if (!r) {
            o.state = d;
          }
        }
        if (u.length > 1e6) {
          u = [];
          throw new Error();
        }
      }, false);
    }
  }
  return s;
}
function B(e) {
  if (!e.fn) {
    return;
  }
  y(e, true);
  const s = v;
  K(e, e.value, s);
}
function K(e, s, t) {
  let n;
  const i = h;
  const o = f;
  f = h = e;
  try {
    n = e.fn(s);
  } catch (r) {
    if (e.pure) {
      e.state = d;
      if (e.owned) {
        e.owned.forEach((l) => y(l));
      }
      e.owned = null;
    }
    e.updatedAt = t + 1;
    return R(r);
  } finally {
    f = o;
    h = i;
  }
  if (!e.updatedAt || e.updatedAt <= t) {
    if (e.updatedAt != null && "observers" in e) {
      _(e, n);
    } else {
      e.value = n;
    }
    e.updatedAt = t;
  }
}
function X(e, s, t, n = d, i) {
  const o = {
    fn: e,
    state: n,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: s,
    owner: h,
    context: h ? h.context : null,
    pure: t
  };
  if (h !== null) {
    if (h !== L) {
      if (h.owned) {
        h.owned.push(o);
      } else {
        h.owned = [o];
      }
    }
  }
  return o;
}
function V(e) {
  if (e.state === 0) {
    return;
  }
  if (e.state === x) {
    return C(e);
  }
  if (e.suspense && N(e.suspense.inFallback)) {
    return e.suspense.effects.push(e);
  }
  const s = [e];
  while ((e = e.owner) && (!e.updatedAt || e.updatedAt < v)) {
    if (e.state) {
      s.push(e);
    }
  }
  for (let t = s.length - 1; t >= 0; t--) {
    e = s[t];
    if (e.state === d) {
      B(e);
    } else if (e.state === x) {
      const n = u;
      u = null;
      b(() => C(e, s[0]), false);
      u = n;
    }
  }
}
function b(e, s) {
  if (u) {
    return e();
  }
  let t = false;
  if (!s) {
    u = [];
  }
  if (w) {
    t = true;
  } else {
    w = [];
  }
  v++;
  try {
    const n = e();
    Z(t);
    return n;
  } catch (n) {
    if (!t) {
      w = null;
    }
    u = null;
    R(n);
  }
}
function Z(e) {
  if (u) {
    q(u);
    u = null;
  }
  if (e) {
    return;
  }
  const s = w;
  w = null;
  if (s.length) {
    b(() => P(s), false);
  }
}
function q(e) {
  for (let s = 0; s < e.length; s++) {
    V(e[s]);
  }
}
function C(e, s) {
  e.state = 0;
  for (let t = 0; t < e.sources.length; t += 1) {
    const n = e.sources[t];
    if (n.sources) {
      const i = n.state;
      if (i === d) {
        if (n !== s && (!n.updatedAt || n.updatedAt < v)) {
          V(n);
        }
      } else if (i === x) {
        C(n, s);
      }
    }
  }
}
function F(e) {
  for (let s = 0; s < e.observers.length; s += 1) {
    const t = e.observers[s];
    if (!t.state) {
      t.state = x;
      if (t.pure) {
        u.push(t);
      } else {
        w.push(t);
      }
      if (t.observers) {
        F(t);
      }
    }
  }
}
function y(e, s) {
  let t;
  if (e.sources) {
    while (e.sources.length) {
      const n = e.sources.pop();
      const i = e.sourceSlots.pop();
      const o = n.observers;
      if (o && o.length) {
        if (!n.observerSlots || n.observerSlots.length !== o.length) {
          M(n);
        }
        const r = o.pop();
        const l = n.observerSlots.pop();
        if (i < o.length) {
          r.sourceSlots[l] = i;
          o[i] = r;
          n.observerSlots[i] = l;
        }
      }
      if (!s) {
        if (o) {
          o.length;
        }
      }
    }
  }
  if (e.tOwned) {
    for (t = e.tOwned.length - 1; t >= 0; t--) {
      y(e.tOwned[t]);
    }
    delete e.tOwned;
  }
  if (e.owned) {
    for (t = e.owned.length - 1; t >= 0; t--) {
      y(e.owned[t]);
    }
    e.owned = null;
  }
  if (e.cleanups) {
    for (t = e.cleanups.length - 1; t >= 0; t--) {
      e.cleanups[t]();
    }
    e.cleanups = null;
  }
  e.state = 0;
}
function k(e) {
  if (e instanceof Error) {
    return e;
  } else {
    return new Error(typeof e == "string" ? e : "Unknown error", { cause: e });
  }
}
function R(e, s = h) {
  throw k(e);
}
function z(e, s) {
  return N(() => e(s || {}));
}
function ee(e, s, t) {
  const n = t.length;
  let i = s.length;
  let o = n;
  let r = 0;
  let l = 0;
  const c = s[i - 1].nextSibling;
  let p = null;
  while (r < i || l < o) {
    if (s[r] === t[l]) {
      r++;
      l++;
      continue;
    }
    while (s[i - 1] === t[o - 1]) {
      i--;
      o--;
    }
    if (i === r) {
      const a = o < n ? l ? t[l - 1].nextSibling : t[o - l] : c;
      while (l < o) {
        e.insertBefore(t[l++], a);
      }
    } else if (o === l) {
      while (r < i) {
        if (!p || !p.has(s[r])) {
          s[r].remove();
        }
        r++;
      }
    } else if (s[r] === t[o - 1] && t[l] === s[i - 1]) {
      const a = s[--i].nextSibling;
      e.insertBefore(t[l++], s[r++].nextSibling);
      e.insertBefore(t[--o], a);
      s[i] = t[o];
    } else {
      if (!p) {
        p = new Map();
        let g = l;
        while (g < o) {
          p.set(t[g], g++);
        }
      }
      const a = p.get(s[r]);
      if (a != null) {
        if (l < a && a < o) {
          let g = r;
          let m = 1;
          let D;
          while (++g < i && g < o && (D = p.get(s[g])) != null && D === a + m) {
            m++;
          }
          if (m > a - l) {
            const j = s[r];
            while (l < a) {
              e.insertBefore(t[l++], j);
            }
          } else {
            e.replaceChild(t[l++], s[r++]);
          }
        } else {
          r++;
        }
      } else {
        s[r++].remove();
      }
    }
  }
}
function te(e, s, t, n = {}) {
  let i;
  Q((o) => {
    i = o;
    if (s === document) {
      e();
    } else {
      $(s, e(), s.firstChild ? null : undefined, t);
    }
  }, n.owner);
  return () => {
    i();
    s.textContent = "";
  };
}
function se(e, s, t, n) {
  let i;
  const o = () => {
    const l = document.createElement("template");
    l.innerHTML = e;
    return l.content.firstChild;
  };
  const r = () => (i ||= o()).cloneNode(true);
  r.cloneNode = r;
  return r;
}
function $(e, s, t, n) {
  if (t !== undefined && !n) {
    n = [];
  }
  if (typeof s != "function") {
    return E(e, s, n, t);
  }
  T((i) => E(e, s(), i, t), n);
}
function E(e, s, t, n, i) {
  while (typeof t == "function") {
    t = t();
  }
  if (s === t) {
    return t;
  }
  const o = typeof s;
  const r = n !== undefined;
  e = r && t[0] && t[0].parentNode || e;
  if (o === "string" || o === "number") {
    if (o === "number" && (s = s.toString(), s === t)) {
      return t;
    }
    if (r) {
      let l = t[0];
      if (l && l.nodeType === 3) {
        if (l.data !== s) {
          l.data = s;
        }
      } else {
        l = document.createTextNode(s);
      }
      t = S(e, t, n, l);
    } else if (t !== "" && typeof t == "string") {
      t = e.firstChild.data = s;
    } else {
      t = e.textContent = s;
    }
  } else if (s == null || o === "boolean") {
    t = S(e, t, n);
  } else {
    if (o === "function") {
      T(() => {
        let l = s();
        while (typeof l == "function") {
          l = l();
        }
        t = E(e, l, t, n);
      });
      return () => t;
    }
    if (Array.isArray(s)) {
      const l = [];
      const c = t && Array.isArray(t);
      if (O(l, s, t, i)) {
        T(() => t = E(e, l, t, n, true));
        return () => t;
      }
      if (l.length === 0) {
        t = S(e, t, n);
        if (r) {
          return t;
        }
      } else if (c) {
        if (t.length === 0) {
          U(e, l, n);
        } else {
          ee(e, t, l);
        }
      } else {
        if (t) {
          S(e);
        }
        U(e, l);
      }
      t = l;
    } else if (s.nodeType) {
      if (Array.isArray(t)) {
        if (r) {
          return t = S(e, t, n, s);
        }
        S(e, t, null, s);
      } else if (t == null || t === "" || !e.firstChild) {
        e.appendChild(s);
      } else {
        e.replaceChild(s, e.firstChild);
      }
      t = s;
    }
  }
  return t;
}
function O(e, s, t, n) {
  let i = false;
  for (let o = 0, r = s.length; o < r; o++) {
    let l = s[o];
    const c = t && t[e.length];
    let p;
    if (l != null && l !== true && l !== false) {
      if ((p = typeof l) == "object" && l.nodeType) {
        e.push(l);
      } else if (Array.isArray(l)) {
        i = O(e, l, c) || i;
      } else if (p === "function") {
        if (n) {
          while (typeof l == "function") {
            l = l();
          }
          i = O(e, Array.isArray(l) ? l : [l], Array.isArray(c) ? c : [c]) || i;
        } else {
          e.push(l);
          i = true;
        }
      } else {
        const a = String(l);
        if (c && c.nodeType === 3 && c.data === a) {
          e.push(c);
        } else {
          e.push(document.createTextNode(a));
        }
      }
    }
  }
  return i;
}
function U(e, s, t = null) {
  for (let n = 0, i = s.length; n < i; n++) {
    e.insertBefore(s[n], t);
  }
}
function S(e, s, t, n) {
  if (t === undefined) {
    return e.textContent = "";
  }
  const i = n || document.createTextNode("");
  if (s.length) {
    let o = false;
    for (let r = s.length - 1; r >= 0; r--) {
      const l = s[r];
      if (i !== l) {
        const c = l.parentNode === e;
        if (!o && !r) {
          if (c) {
            e.replaceChild(i, l);
          } else {
            e.insertBefore(i, t);
          }
        } else if (c) {
          l.remove();
        }
      } else {
        o = true;
      }
    }
  } else {
    e.insertBefore(i, t);
  }
  return [i];
}
const ne = (e) => {
  console.log("safe");
  return e();
};
const le = (e) => {
  console.log("sched");
  e();
};
function ie(e) {
  le(() => ne(e));
}
var oe = se("<div><p>");
function re() {
  const [e] = Y(1);
  ie(() => console.log("later ran", e()));
  return (() => {
    const s = oe();
    const t = s.firstChild;
    $(t, e);
    return s;
  })();
}
te(() => z(re, {}), document.body);