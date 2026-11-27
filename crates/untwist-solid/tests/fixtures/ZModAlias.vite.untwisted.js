let G = M;
const y = 1;
const x = 2;
const I = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var u = null;
let S = null;
let P = null;
let a = null;
let h = null;
let g = null;
let E = 0;
function $(e, n) {
  const t = a;
  const l = u;
  const i = e.length === 0;
  const o = n === undefined ? l : n;
  const f = i ? I : {
    owned: null,
    cleanups: null,
    context: o ? o.context : null,
    owner: o
  };
  const s = i ? e : () => e(H(f));
  u = f;
  a = null;
  try {
    return b(s, true);
  } finally {
    a = t;
    u = l;
  }
}
function H(e) {
  return () => B(() => A(e));
}
function T(e, n, t) {
  const l = q(e, n, false, y);
  U(l);
}
function B(e) {
  if (a === null) {
    return e();
  }
  const n = a;
  a = null;
  try {
    return e();
  } finally {
    a = n;
  }
}
function Q(e) {
  e.observerSlots = e.observers.map((n, t) => {
    const l = n.sources;
    if (!l) {
      return -1;
    }
    for (let i = l.length - 1; i >= 0; i--) {
      if (l[i] === e && n.sourceSlots && n.sourceSlots[i] === t) {
        return i;
      }
    }
    return l.lastIndexOf(e);
  });
}
function W(e, n, t) {
  const l = e.value;
  if (!e.comparator || !e.comparator(l, n)) {
    e.value = n;
    if (e.observers && e.observers.length) {
      b(() => {
        for (let i = 0; i < e.observers.length; i += 1) {
          const o = e.observers[i];
          const f = S && S.running;
          if (f) {
            S.disposed.has(o);
          }
          if (f ? !o.tState : !o.state) {
            if (o.pure) {
              h.push(o);
            } else {
              g.push(o);
            }
            if (o.observers) {
              V(o);
            }
          }
          if (!f) {
            o.state = y;
          }
        }
        if (h.length > 1e6) {
          h = [];
          throw new Error();
        }
      }, false);
    }
  }
  return n;
}
function U(e) {
  if (!e.fn) {
    return;
  }
  A(e, true);
  const n = E;
  Z(e, e.value, n);
}
function Z(e, n, t) {
  let l;
  const i = u;
  const o = a;
  a = u = e;
  try {
    l = e.fn(n);
  } catch (f) {
    if (e.pure) {
      e.state = y;
      if (e.owned) {
        e.owned.forEach((s) => A(s));
      }
      e.owned = null;
    }
    e.updatedAt = t + 1;
    return _(f);
  } finally {
    a = o;
    u = i;
  }
  if (!e.updatedAt || e.updatedAt <= t) {
    if (e.updatedAt != null && "observers" in e) {
      W(e, l);
    } else {
      e.value = l;
    }
    e.updatedAt = t;
  }
}
function q(e, n, t, l = y, i) {
  const o = {
    fn: e,
    state: l,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: n,
    owner: u,
    context: u ? u.context : null,
    pure: t
  };
  if (u !== null) {
    if (u !== I) {
      if (u.owned) {
        u.owned.push(o);
      } else {
        u.owned = [o];
      }
    }
  }
  return o;
}
function L(e) {
  if (e.state === 0) {
    return;
  }
  if (e.state === x) {
    return N(e);
  }
  if (e.suspense && B(e.suspense.inFallback)) {
    return e.suspense.effects.push(e);
  }
  const n = [e];
  while ((e = e.owner) && (!e.updatedAt || e.updatedAt < E)) {
    if (e.state) {
      n.push(e);
    }
  }
  for (let t = n.length - 1; t >= 0; t--) {
    e = n[t];
    if (e.state === y) {
      U(e);
    } else if (e.state === x) {
      const l = h;
      h = null;
      b(() => N(e, n[0]), false);
      h = l;
    }
  }
}
function b(e, n) {
  if (h) {
    return e();
  }
  let t = false;
  if (!n) {
    h = [];
  }
  if (g) {
    t = true;
  } else {
    g = [];
  }
  E++;
  try {
    const l = e();
    J(t);
    return l;
  } catch (l) {
    if (!t) {
      g = null;
    }
    h = null;
    _(l);
  }
}
function J(e) {
  if (h) {
    M(h);
    h = null;
  }
  if (e) {
    return;
  }
  const n = g;
  g = null;
  if (n.length) {
    b(() => G(n), false);
  }
}
function M(e) {
  for (let n = 0; n < e.length; n++) {
    L(e[n]);
  }
}
function N(e, n) {
  e.state = 0;
  for (let t = 0; t < e.sources.length; t += 1) {
    const l = e.sources[t];
    if (l.sources) {
      const i = l.state;
      if (i === y) {
        if (l !== n && (!l.updatedAt || l.updatedAt < E)) {
          L(l);
        }
      } else if (i === x) {
        N(l, n);
      }
    }
  }
}
function V(e) {
  for (let n = 0; n < e.observers.length; n += 1) {
    const t = e.observers[n];
    if (!t.state) {
      t.state = x;
      if (t.pure) {
        h.push(t);
      } else {
        g.push(t);
      }
      if (t.observers) {
        V(t);
      }
    }
  }
}
function A(e, n) {
  let t;
  if (e.sources) {
    while (e.sources.length) {
      const l = e.sources.pop();
      const i = e.sourceSlots.pop();
      const o = l.observers;
      if (o && o.length) {
        if (!l.observerSlots || l.observerSlots.length !== o.length) {
          Q(l);
        }
        const f = o.pop();
        const s = l.observerSlots.pop();
        if (i < o.length) {
          f.sourceSlots[s] = i;
          o[i] = f;
          l.observerSlots[i] = s;
        }
      }
      if (!n) {
        if (o) {
          o.length;
        }
      }
    }
  }
  if (e.tOwned) {
    for (t = e.tOwned.length - 1; t >= 0; t--) {
      A(e.tOwned[t]);
    }
    delete e.tOwned;
  }
  if (e.owned) {
    for (t = e.owned.length - 1; t >= 0; t--) {
      A(e.owned[t]);
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
function K(e) {
  if (e instanceof Error) {
    return e;
  } else {
    return new Error(typeof e == "string" ? e : "Unknown error", { cause: e });
  }
}
function _(e, n = u) {
  throw K(e);
}
function X(e, n) {
  return B(() => e(n || {}));
}
function Y(e, n, t) {
  const l = t.length;
  let i = n.length;
  let o = l;
  let f = 0;
  let s = 0;
  const r = n[i - 1].nextSibling;
  let p = null;
  while (f < i || s < o) {
    if (n[f] === t[s]) {
      f++;
      s++;
      continue;
    }
    while (n[i - 1] === t[o - 1]) {
      i--;
      o--;
    }
    if (i === f) {
      const c = o < l ? s ? t[s - 1].nextSibling : t[o - s] : r;
      while (s < o) {
        e.insertBefore(t[s++], c);
      }
    } else if (o === s) {
      while (f < i) {
        if (!p || !p.has(n[f])) {
          n[f].remove();
        }
        f++;
      }
    } else if (n[f] === t[o - 1] && t[s] === n[i - 1]) {
      const c = n[--i].nextSibling;
      e.insertBefore(t[s++], n[f++].nextSibling);
      e.insertBefore(t[--o], c);
      n[i] = t[o];
    } else {
      if (!p) {
        p = new Map();
        let w = s;
        while (w < o) {
          p.set(t[w], w++);
        }
      }
      const c = p.get(n[f]);
      if (c != null) {
        if (s < c && c < o) {
          let w = f;
          let m = 1;
          let O;
          while (++w < i && w < o && (O = p.get(n[w])) != null && O === c + m) {
            m++;
          }
          if (m > c - s) {
            const F = n[f];
            while (s < c) {
              e.insertBefore(t[s++], F);
            }
          } else {
            e.replaceChild(t[s++], n[f++]);
          }
        } else {
          f++;
        }
      } else {
        n[f++].remove();
      }
    }
  }
}
function k(e, n, t, l = {}) {
  let i;
  $((o) => {
    i = o;
    if (n === document) {
      e();
    } else {
      R(n, e(), n.firstChild ? null : undefined, t);
    }
  }, l.owner);
  return () => {
    i();
    n.textContent = "";
  };
}
function z(e, n, t, l) {
  let i;
  const o = () => {
    const s = document.createElement("template");
    s.innerHTML = e;
    return s.content.firstChild;
  };
  const f = () => (i ||= o()).cloneNode(true);
  f.cloneNode = f;
  return f;
}
function R(e, n, t, l) {
  if (t !== undefined && !l) {
    l = [];
  }
  if (typeof n != "function") {
    return C(e, n, l, t);
  }
  T((i) => C(e, n(), i, t), l);
}
function C(e, n, t, l, i) {
  while (typeof t == "function") {
    t = t();
  }
  if (n === t) {
    return t;
  }
  const o = typeof n;
  const f = l !== undefined;
  e = f && t[0] && t[0].parentNode || e;
  if (o === "string" || o === "number") {
    if (o === "number" && (n = n.toString(), n === t)) {
      return t;
    }
    if (f) {
      let s = t[0];
      if (s && s.nodeType === 3) {
        if (s.data !== n) {
          s.data = n;
        }
      } else {
        s = document.createTextNode(n);
      }
      t = d(e, t, l, s);
    } else if (t !== "" && typeof t == "string") {
      t = e.firstChild.data = n;
    } else {
      t = e.textContent = n;
    }
  } else if (n == null || o === "boolean") {
    t = d(e, t, l);
  } else {
    if (o === "function") {
      T(() => {
        let s = n();
        while (typeof s == "function") {
          s = s();
        }
        t = C(e, s, t, l);
      });
      return () => t;
    }
    if (Array.isArray(n)) {
      const s = [];
      const r = t && Array.isArray(t);
      if (v(s, n, t, i)) {
        T(() => t = C(e, s, t, l, true));
        return () => t;
      }
      if (s.length === 0) {
        t = d(e, t, l);
        if (f) {
          return t;
        }
      } else if (r) {
        if (t.length === 0) {
          D(e, s, l);
        } else {
          Y(e, t, s);
        }
      } else {
        if (t) {
          d(e);
        }
        D(e, s);
      }
      t = s;
    } else if (n.nodeType) {
      if (Array.isArray(t)) {
        if (f) {
          return t = d(e, t, l, n);
        }
        d(e, t, null, n);
      } else if (t == null || t === "" || !e.firstChild) {
        e.appendChild(n);
      } else {
        e.replaceChild(n, e.firstChild);
      }
      t = n;
    }
  }
  return t;
}
function v(e, n, t, l) {
  let i = false;
  for (let o = 0, f = n.length; o < f; o++) {
    let s = n[o];
    const r = t && t[e.length];
    let p;
    if (s != null && s !== true && s !== false) {
      if ((p = typeof s) == "object" && s.nodeType) {
        e.push(s);
      } else if (Array.isArray(s)) {
        i = v(e, s, r) || i;
      } else if (p === "function") {
        if (l) {
          while (typeof s == "function") {
            s = s();
          }
          i = v(e, Array.isArray(s) ? s : [s], Array.isArray(r) ? r : [r]) || i;
        } else {
          e.push(s);
          i = true;
        }
      } else {
        const c = String(s);
        if (r && r.nodeType === 3 && r.data === c) {
          e.push(r);
        } else {
          e.push(document.createTextNode(c));
        }
      }
    }
  }
  return i;
}
function D(e, n, t = null) {
  for (let l = 0, i = n.length; l < i; l++) {
    e.insertBefore(n[l], t);
  }
}
function d(e, n, t, l) {
  if (t === undefined) {
    return e.textContent = "";
  }
  const i = l || document.createTextNode("");
  if (n.length) {
    let o = false;
    for (let f = n.length - 1; f >= 0; f--) {
      const s = n[f];
      if (i !== s) {
        const r = s.parentNode === e;
        if (!o && !f) {
          if (r) {
            e.replaceChild(i, s);
          } else {
            e.insertBefore(i, t);
          }
        } else if (r) {
          s.remove();
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
var ee = z("<p>");
const j = [];
const te = j;
te.push(1, 2);
function ne() {
  return (() => {
    const e = ee();
    R(e, () => j.join());
    return e;
  })();
}
k(() => X(ne, {}), document.body);