use untwist_eval::builtins::object::own_keys;
use untwist_eval::builtins::regexp::source_text;
use untwist_eval::coerce::number_to_string;
use untwist_eval::inspect::{intrinsic_label, native_label, natural_prototype};
use untwist_eval::object::own_property;
use untwist_eval::ops::string_key;
use untwist_eval::source::{function_source, is_method_code};
use untwist_eval::unmodelled::is_intrinsic_object;
use untwist_eval::value::{JsString, ObjKind, ObjRef, PropKey, Value};

const DEPTH_CAP: usize = 4;
const BOUND_CHAIN_CAP: usize = 16;

pub type Rendered = Result<String, String>;

pub const NODE_RENDER: &str = r#"
var NAT = new Map(), PRO = new Map();
function nameIntrinsics(natives, roots) {
  for (var i = 0; i < natives.length; i++) { try { var f = (0, eval)(natives[i][0]); if (typeof f === 'function' && !NAT.has(f)) NAT.set(f, natives[i][1]); } catch (e) {} }
  var protos = ['DOMException', 'Object', 'Array', 'Function', 'String', 'Number', 'Boolean', 'RegExp', 'Error', 'TypeError', 'RangeError', 'SyntaxError', 'ReferenceError', 'URIError', 'EvalError', 'Symbol', 'Map', 'Set', 'Promise', 'Date'];
  for (var j = 0; j < protos.length; j++) PRO.set(globalThis[protos[j]].prototype, protos[j] + '.prototype');
  PRO.set(Object.getPrototypeOf(function* () {}), 'GeneratorFunction.prototype');
  PRO.set(Object.getPrototypeOf(async function () {}), 'AsyncFunction.prototype');
  PRO.set(Object.getPrototypeOf(async function* () {}), 'AsyncGeneratorFunction.prototype');
  for (var k = 0; k < roots.length; k++) { var r = globalThis[roots[k]]; if (r !== null && typeof r === 'object' && !PRO.has(r)) PRO.set(r, roots[k]); }
}
function isIndex(k) { return String(k >>> 0) === k && k !== '4294967295'; }
function plabel(p, d) { if (p === null) return 'n'; if (PRO.has(p)) return PRO.get(p); return 'U' + R(p, d + 1); }
function sfx(v, natural, d, skip) {
  var pl = plabel(Object.getPrototypeOf(v), d), out = pl === natural ? '' : '<' + pl + '>';
  var ks = Object.keys(v).filter(function (k) { return !skip(k); });
  if (ks.length) out += '{' + ks.map(function (k) { return R(k, d + 1) + '=' + own(v, k, d + 1); }).join(',') + '}';
  return out;
}
function none() { return false; }
function own(v, k, d) { var desc = Object.getOwnPropertyDescriptor(v, k); return desc && 'value' in desc ? R(desc.value, d) : 'acc'; }
function fsrc(v) {
  if (NAT.has(v)) return 'N' + NAT.get(v);
  var s = Function.prototype.toString.call(v);
  if (s === 'function () { [native code] }') return 'B';
  if (/\{ \[native code\] \}$/.test(s)) return 'N?' + v.name;
  if (/^(async\s+)?function\b/.test(s) || /^(async\s*)?(\([^)]*\)|[\w$]+)\s*=>/.test(s)) return s;
  return 'M';
}
function R(v, d) {
  if (d > 4) return '~';
  if (v === undefined) return 'u';
  if (v === null) return 'n';
  if (typeof v === 'boolean') return v ? 'b1' : 'b0';
  if (typeof v === 'number') return 'd' + (Object.is(v, -0) ? '-0' : String(v));
  if (typeof v === 'string') { var u = []; for (var i = 0; i < v.length; i++) u.push(v.charCodeAt(i).toString(16)); return 's' + u.join('.'); }
  if (typeof v !== 'object' && typeof v !== 'function') return 'X' + typeof v;
  var tag = Object.prototype.toString.call(v).slice(8, -1);
  if (PRO.has(v)) return 'I:' + PRO.get(v);
  if (typeof v === 'function' && NAT.has(v)) return 'F' + tag + ':' + fsrc(v);
  if (typeof v === 'function') return 'F' + tag + ':' + fsrc(v) + sfx(v, tag + '.prototype', d, function (k) { return k === 'prototype'; });
  if (tag === 'RegExp') return 'R' + RegExp.prototype.toString.call(v) + ':' + R(v.lastIndex, d + 1) + sfx(v, 'RegExp.prototype', d, none);
  if (tag === 'Error' || v instanceof Error) { var ep = plabel(Object.getPrototypeOf(v), d); return 'E' + ep + sfx(v, ep, d, none); }
  if (Array.isArray(v)) { var a = []; for (var i = 0; i < v.length; i++) a.push(i in v ? own(v, String(i), d + 1) : 'h'); return 'A[' + a.join(',') + ']' + sfx(v, 'Array.prototype', d, isIndex); }
  if (tag === 'Arguments') { var g = []; for (var i = 0; i < v.length; i++) g.push(own(v, String(i), d + 1)); return 'G[' + g.join(',') + ']' + sfx(v, 'Object.prototype', d, isIndex); }
  var pl = plabel(Object.getPrototypeOf(v), d);
  var o = []; var ks = Object.keys(v); for (var i = 0; i < ks.length; i++) o.push(R(ks[i], d + 1) + '=' + own(v, ks[i], d + 1));
  return 'O' + (pl === 'Object.prototype' ? '' : '<' + pl + '>') + '{' + o.join(',') + '}';
}
"#;

pub struct Renderer<'s> {
    pub source: &'s str,
}

fn unrendered(why: &str) -> Rendered {
    Err(format!("unrendered {why}"))
}

fn string_text(text: &JsString) -> String {
    let units: Vec<String> = text.units().iter().map(|u| format!("{u:x}")).collect();
    format!("s{}", units.join("."))
}

fn name_of(key: &PropKey) -> JsString {
    match key {
        PropKey::Name(name) => name.clone(),
        PropKey::Index(index) => JsString::from_str(&index.to_string()),
    }
}

fn is_index_key(key: &PropKey) -> bool {
    matches!(key, PropKey::Index(index) if *index != u32::MAX)
}

impl Renderer<'_> {
    pub fn render(&self, value: &Value<'_>, depth: usize) -> Rendered {
        if depth > DEPTH_CAP {
            return Ok("~".to_string());
        }
        Ok(match value {
            Value::Undefined => "u".to_string(),
            Value::Null => "n".to_string(),
            Value::Bool(b) => if *b { "b1" } else { "b0" }.to_string(),
            Value::Number(n) if *n == 0.0 && n.is_sign_negative() => "d-0".to_string(),
            Value::Number(n) => format!("d{}", number_to_string(*n)),
            Value::Str(text) => string_text(text),
            Value::Object(obj) => return self.object(value, obj, depth),
        })
    }

    fn object(&self, value: &Value<'_>, obj: &ObjRef<'_>, depth: usize) -> Rendered {
        if value.is_callable() {
            return self.function(obj, depth);
        }
        if is_intrinsic_object(obj) {
            return match intrinsic_label(obj) {
                Some(label) => Ok(format!("I:{label}")),
                None => unrendered("intrinsic object"),
            };
        }
        let borrowed = obj.borrow();
        match &borrowed.kind {
            ObjKind::RegExp(data) => {
                let text = format!(
                    "R{}:d{}",
                    source_text(data).to_lossy_string(),
                    data.last_index.get()
                );
                drop(borrowed);
                Ok(text + &self.suffix(obj, depth, |_| false)?)
            }
            ObjKind::Error(_) => {
                drop(borrowed);
                let proto = self.proto_label(obj, depth)?;
                let extra = self.extra_keys(obj, depth, |key| is_name(key, "message"))?;
                Ok(format!("E{proto}{extra}"))
            }
            ObjKind::Array(items) | ObjKind::Arguments(items) => {
                let letter = if matches!(borrowed.kind, ObjKind::Array(_)) {
                    "A"
                } else {
                    "G"
                };
                let items = items.clone();
                drop(borrowed);
                self.indexed(letter, &items, obj, depth)
            }
            ObjKind::MappedArguments(mapped) => {
                let current = mapped.current_items();
                drop(borrowed);
                self.indexed("G", &current, obj, depth)
            }
            ObjKind::Plain(_) => {
                drop(borrowed);
                self.plain(value, obj, depth)
            }
            _ => unrendered("object kind"),
        }
    }

    fn indexed(
        &self,
        letter: &str,
        items: &[Value<'_>],
        obj: &ObjRef<'_>,
        depth: usize,
    ) -> Rendered {
        let parts: Result<Vec<String>, String> = items
            .iter()
            .map(|item| self.render(item, depth + 1))
            .collect();
        let head = format!("{letter}[{}]", parts?.join(","));
        Ok(head + &self.suffix(obj, depth, is_index_key)?)
    }

    fn function(&self, obj: &ObjRef<'_>, depth: usize) -> Rendered {
        let tag = self.function_tag(obj)?;
        let text = match &obj.borrow().kind {
            ObjKind::Function(closure) if is_method_code(self.source, closure.code) => {
                "M".to_string()
            }
            ObjKind::Function(closure) => function_source(self.source, closure.code),
            ObjKind::Bound(_) => "B".to_string(),
            ObjKind::Native(id, _) => {
                return Ok(match native_label(*id) {
                    Some(label) => format!("F{tag}:N{}", canonical(label)),
                    None => format!("F{tag}:N?{}", hostile_name(obj)),
                })
            }
            _ => return unrendered("callable kind"),
        };
        let suffix = self.suffix(obj, depth, |key| is_name(key, "prototype"))?;
        Ok(format!("F{tag}:{text}{suffix}"))
    }

    fn function_tag(&self, obj: &ObjRef<'_>) -> Result<&'static str, String> {
        let mut current = obj.clone();
        for _ in 0..BOUND_CHAIN_CAP {
            let next = match &current.borrow().kind {
                ObjKind::Function(closure) => return Ok(closure.class_tag()),
                ObjKind::Bound(bound) => bound.target.as_object().cloned(),
                _ => return Ok("Function"),
            };
            let Some(next) = next else {
                return Err("unrendered bound target".to_string());
            };
            current = next;
        }
        Err("unrendered bound chain".to_string())
    }

    fn plain(&self, value: &Value<'_>, obj: &ObjRef<'_>, depth: usize) -> Rendered {
        let keys = own_keys(value).map_err(|bail| format!("unrendered keys: {bail}"))?;
        let mut parts = Vec::with_capacity(keys.len());
        for key in keys {
            let member = own_property(obj, &string_key(key.clone())).unwrap_or(Value::Undefined);
            parts.push(format!(
                "{}={}",
                self.render(&Value::Str(key), depth + 1)?,
                self.render(&member, depth + 1)?
            ));
        }
        let proto = self.proto_label(obj, depth)?;
        let shown = if proto == "Object.prototype" {
            String::new()
        } else {
            format!("<{proto}>")
        };
        Ok(format!("O{shown}{{{}}}", parts.join(",")))
    }

    fn proto_label(&self, obj: &ObjRef<'_>, depth: usize) -> Rendered {
        let proto = obj.borrow().proto.clone();
        match proto {
            Some(proto) => match intrinsic_label(&proto) {
                Some(label) => Ok(label),
                None => Ok(format!(
                    "U{}",
                    self.render(&Value::Object(proto), depth + 1)?
                )),
            },
            None => {
                natural_prototype(obj).ok_or_else(|| "unrendered implicit prototype".to_string())
            }
        }
    }

    fn suffix(&self, obj: &ObjRef<'_>, depth: usize, skip: impl Fn(&PropKey) -> bool) -> Rendered {
        let natural = natural_prototype(obj).unwrap_or_default();
        let proto = self.proto_label(obj, depth)?;
        let mut out = if proto == natural {
            String::new()
        } else {
            format!("<{proto}>")
        };
        out.push_str(&self.extra_keys(obj, depth, skip)?);
        Ok(out)
    }

    fn extra_keys(
        &self,
        obj: &ObjRef<'_>,
        depth: usize,
        skip: impl Fn(&PropKey) -> bool,
    ) -> Rendered {
        let mut entries: Vec<(PropKey, Value<'_>)> = obj
            .borrow()
            .props
            .iter()
            .map(|(key, value)| (key.key(), value))
            .filter(|(key, _)| !skip(key))
            .map(|(key, value)| (key, value.clone()))
            .collect();
        entries.sort_by_key(|(key, _)| match key {
            PropKey::Index(index) if *index != u32::MAX => (0, *index),
            _ => (1, 0),
        });
        if entries.is_empty() {
            return Ok(String::new());
        }
        let mut parts = Vec::with_capacity(entries.len());
        for (key, member) in entries {
            parts.push(format!(
                "{}={}",
                self.render(&Value::Str(name_of(&key)), depth + 1)?,
                self.render(&member, depth + 1)?
            ));
        }
        Ok(format!("{{{}}}", parts.join(",")))
    }
}

const ALIASES: &[(&str, &str)] = &[
    ("Number.parseInt", "parseInt"),
    ("Number.parseFloat", "parseFloat"),
    ("String.prototype.trimLeft", "String.prototype.trimStart"),
    ("String.prototype.trimRight", "String.prototype.trimEnd"),
];

pub fn canonical(label: &str) -> &str {
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == label)
        .map_or(label, |(_, target)| target)
}

fn is_name(key: &PropKey, name: &str) -> bool {
    matches!(key, PropKey::Name(text) if text.to_lossy_string() == name)
}

fn hostile_name(obj: &ObjRef<'_>) -> String {
    match own_property(obj, &PropKey::Name(JsString::from_str("name"))) {
        Some(Value::Str(text)) => text.to_lossy_string(),
        _ => String::new(),
    }
}
