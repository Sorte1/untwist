use oxc_ast::ast::{Expression, ObjectProperty, PropertyKey, PropertyKind};

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::builtins::args::{
    adopt_array, adopt_charge, argument, grow_object, heap_checkpoint, live_array, reserve_array,
    text_bytes, write_property,
};
use crate::builtins::{define_builtin, Globals, NativeFn, NativeTable, Realm};
use crate::coerce::number_to_js_string;
use crate::heap::Charge;
use crate::host::Host;
use crate::object::{own_property, Descriptor};
use crate::ops::{number_key, string_key, to_js_string};
use crate::value::{
    js_string_from_literal, Eval, JsString, ObjKind, ObjRef, PropKey, StoredKey, Value,
};

pub fn static_property_key(key: &PropertyKey<'_>) -> Option<PropKey> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(string_key(JsString::from_str(id.name.as_str()))),
        PropertyKey::PrivateIdentifier(_) => None,
        other => match other.as_expression()? {
            Expression::StringLiteral(literal) => {
                Some(string_key(js_string_from_literal(literal.as_ref())))
            }
            Expression::NumericLiteral(literal) => Some(number_key(literal.value)),
            _ => None,
        },
    }
}

const PROTO_KEY: &str = "__proto__";

pub static ACCESSOR_IN_LITERAL: &str = "an accessor in an object literal";

pub fn unmodelled_literal_entry(entry: &ObjectProperty<'_>) -> Option<&'static &'static str> {
    if entry.kind != PropertyKind::Init {
        return Some(&ACCESSOR_IN_LITERAL);
    }
    let sets_prototype = !entry.computed && !entry.shorthand && !entry.method;
    let proto_key = match &entry.key {
        PropertyKey::StaticIdentifier(id) => id.name.as_str() == PROTO_KEY,
        PropertyKey::StringLiteral(literal) => literal.value.as_str() == PROTO_KEY,
        _ => false,
    };
    if sets_prototype && proto_key {
        return Some(&"a __proto__ key in an object literal");
    }
    None
}

pub fn own_keys<'a>(target: &Value<'a>) -> EvalResult<Vec<JsString>> {
    crate::state::note_key_observation(target);
    crate::unmodelled::refuse_unmodelled_keys(target)?;
    let Some(obj) = target.as_object() else {
        return Ok(primitive_keys(target));
    };
    crate::object::refuse_undefined_array_items(obj)?;
    let borrowed = obj.borrow();
    let mut keys = Vec::new();
    let dense_len = match &borrowed.kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => Some(items.len()),
        ObjKind::MappedArguments(mapped) => Some(mapped.items.len()),
        _ => None,
    };
    if let Some(len) = dense_len {
        for index in 0..len {
            keys.push(number_to_js_string(index as f64));
        }
    }
    let enumerable = || borrowed.props.keys().filter(|key| key.attrs().enumerable());
    let mut indices: Vec<u32> = enumerable().filter_map(array_index).collect();
    indices.sort_unstable();
    keys.extend(
        indices
            .into_iter()
            .map(|index| number_to_js_string(f64::from(index))),
    );
    for key in enumerable() {
        match key {
            StoredKey::Name(name, _) => keys.push(name.clone()),
            StoredKey::Index(index, _) if array_index(key).is_none() => {
                keys.push(number_to_js_string(f64::from(*index)));
            }
            StoredKey::Index(..) => {}
        }
    }
    Ok(keys)
}

fn array_index(key: &StoredKey) -> Option<u32> {
    match key {
        StoredKey::Index(index, _) if *index != u32::MAX => Some(*index),
        _ => None,
    }
}

fn keyed<'a>(host: &mut dyn Host<'a>, args: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    let target = argument(args, 0);
    host.refuse_global_object(&target)?;
    if target.is_nullish() {
        return Err(host.throw(
            ErrorKind::TypeError,
            "cannot convert undefined or null to object",
        ));
    }
    Ok(target)
}

fn primitive_keys(target: &Value<'_>) -> Vec<JsString> {
    let Value::Str(text) = target else {
        return Vec::new();
    };
    (0..text.len())
        .map(|index| number_to_js_string(index as f64))
        .collect()
}

fn native_object<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    match argument(args, 0) {
        Value::Undefined | Value::Null => {
            let proto = host.object_prototype();
            Ok(host.new_object(ObjKind::Plain(0), proto))
        }
        object @ Value::Object(_) => Ok(object),
        _ => Err(Bail::Unsupported(&"Object() boxing a primitive").into()),
    }
}

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let object_global = realm.native_value(table, ("Object", ""), native_object);
    let native_id = object_global
        .as_object()
        .and_then(|obj| match obj.borrow().kind {
            ObjKind::Native(id, _) => Some(id),
            _ => None,
        });
    if let Some(id) = native_id {
        realm.mark_constructible(id);
    }
    if let Some(obj) = object_global.as_object().cloned() {
        install_statics(realm, table, &obj);
    }
    globals.define("Object", object_global);
    install_prototype_members(realm, table);
}

fn install_statics<'a>(realm: &mut Realm<'a>, table: &mut NativeTable<'a>, obj: &ObjRef<'a>) {
    let statics: [(&str, NativeFn<'a>); 9] = [
        ("keys", native_keys),
        ("values", native_values),
        ("entries", native_entries),
        ("assign", native_assign),
        ("freeze", native_freeze),
        ("isFrozen", native_is_frozen),
        ("create", native_create),
        ("defineProperty", native_define_property),
        ("getPrototypeOf", native_get_prototype_of),
    ];
    let pure_statics = ["keys", "values", "entries", "isFrozen", "getPrototypeOf"];
    for (name, function) in statics {
        let value = realm.native_value(table, ("Object", name), function);
        if pure_statics.contains(&name) {
            realm.mark_pure_value(&value);
        }
        define_builtin(obj, name, value);
    }
}

fn install_prototype_members<'a>(realm: &mut Realm<'a>, table: &mut NativeTable<'a>) {
    let has_own = realm.native_pure_value(
        table,
        ("Object.prototype", "hasOwnProperty"),
        native_has_own_property,
    );
    realm.define_object_member("hasOwnProperty", has_own);
    let to_string = realm.native_value(
        table,
        ("Object.prototype", "toString"),
        native_object_to_string,
    );
    realm.define_object_member("toString", to_string);
    let value_of = realm.native_value(table, ("Object.prototype", "valueOf"), native_value_of);
    realm.define_object_member("valueOf", value_of);
}

fn native_keys<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let keys = own_keys(&keyed(host, args)?)?;
    let fresh = reserve_fresh_keys(host, &keys)?;
    let keys = keys.into_iter().map(Value::Str).collect::<Vec<_>>();
    let reserved = reserve_array(host, keys.capacity())?;
    let array = adopt_array(host, keys, reserved, None)?;
    adopt_charge(&array, fresh);
    Ok(array)
}

fn reserve_fresh_keys<'a>(
    host: &mut dyn Host<'a>,
    keys: &[JsString],
) -> EvalResult<Option<Charge>> {
    let bytes = keys
        .iter()
        .filter(|key| key.is_unshared())
        .fold(0u64, |sum, key| sum.saturating_add(text_bytes(key.len())));
    host.reserve_live(bytes)
}

fn native_values<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let target = keyed(host, args)?;
    let mut out = Vec::new();
    for (done, key) in own_keys(&target)?.into_iter().enumerate() {
        heap_checkpoint(host, done)?;
        out.push(host.get_property(&target, &string_key(key))?);
    }
    live_array(host, out)
}

fn native_entries<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let target = keyed(host, args)?;
    let mut out = Vec::new();
    for (done, key) in own_keys(&target)?.into_iter().enumerate() {
        heap_checkpoint(host, done)?;
        let fresh = reserve_fresh_keys(host, std::slice::from_ref(&key))?;
        let value = host.get_property(&target, &string_key(key.clone()))?;
        let pair = live_array(host, vec![Value::Str(key), value])?;
        adopt_charge(&pair, fresh);
        out.push(pair);
    }
    live_array(host, out)
}

fn native_assign<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let target = argument(args, 0);
    let Some(destination) = target.as_object().cloned() else {
        if !target.is_nullish() {
            crate::unmodelled::refuse_while_decoding(&"Object.assign onto a primitive")?;
        }
        return Err(host.throw(
            ErrorKind::TypeError,
            "Object.assign target is not an object",
        ));
    };
    for source in args.iter().skip(1) {
        host.refuse_global_object(source)?;
        for key in own_keys(source)? {
            if key.eq_str("__proto__") {
                return Err(Bail::Unsupported(&"Object.assign of an own __proto__ key").into());
            }
            let value = host.get_property(source, &string_key(key.clone()))?;
            if let Err(bail) = write_property(host, &destination, string_key(key), value) {
                return Err(refused_as_type_error(host, bail));
            }
        }
    }
    Ok(target)
}

fn native_freeze<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if crate::object::decoding() {
        return Err(Bail::Unsupported(&"Object.freeze while decoding").into());
    }
    let target = argument(args, 0);
    if let Some(obj) = target.as_object() {
        crate::object::freeze(obj)?;
    }
    Ok(target)
}

fn native_is_frozen<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(crate::object::is_frozen(args.first())))
}

fn native_create<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let proto = argument(args, 0).as_object().cloned();
    if proto.is_none() || !matches!(argument(args, 1), Value::Undefined) {
        crate::unmodelled::refuse_while_decoding(
            &"Object.create with a null prototype or property descriptors",
        )?;
    }
    if let Some(proto) = &proto {
        crate::unmodelled::refuse_unmodelled_prototype(proto)?;
        crate::state::note_prototype(proto);
        crate::object::note_prototype_attributes(proto);
    }
    Ok(host.new_object(ObjKind::Plain(0), proto))
}

fn native_define_property<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let target = argument(args, 0);
    let Some(obj) = target.as_object().cloned() else {
        return Err(host.throw(
            ErrorKind::TypeError,
            "defineProperty target is not an object",
        ));
    };
    let key = string_key(to_js_string(&argument(args, 1), host)?);
    let dense = crate::object::array_length(&obj).is_some();
    if dense && crate::object::is_length_key(&key) {
        return Err(Bail::Unsupported(&"defineProperty on the length of an array").into());
    }
    let descriptor = argument(args, 2);
    let Some(fields) = descriptor.as_object() else {
        return Err(host.throw(ErrorKind::TypeError, "descriptor is not an object"));
    };
    for accessor in ["get", "set"] {
        let key = PropKey::Name(JsString::from_str(accessor));
        let found = crate::object::walk_prototype_chain(fields.clone(), |link| {
            crate::object::has_own_property(link, &key).then_some(())
        })?;
        if found.is_some() {
            return Err(Bail::Unsupported(&"accessor property descriptor").into());
        }
    }
    if dense || crate::object::decoding() {
        return define_plain_data(host, &obj, key, &descriptor).map(|()| target);
    }
    crate::unmodelled::refuse_unmodelled_write(&obj, &key)?;
    let descriptor = read_descriptor(host, &descriptor)?;
    match crate::object::define_own_property(&obj, key, descriptor) {
        Ok(written) => grow_object(host, &obj, written.prop_bytes)?,
        Err(bail) => return Err(refused_as_type_error(host, bail)),
    }
    Ok(target)
}

#[cold]
#[inline(never)]
fn refused_as_type_error<'a>(host: &mut dyn Host<'a>, bail: Bail) -> crate::value::Abrupt<'a> {
    match bail {
        Bail::Refused => host.throw(ErrorKind::TypeError, "a property's attributes refuse it"),
        other => other.into(),
    }
}

fn define_plain_data<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    key: PropKey,
    descriptor: &Value<'a>,
) -> Eval<'a, ()> {
    let Some(fields) = descriptor.as_object() else {
        return Ok(());
    };
    for attribute in ["writable", "enumerable", "configurable"] {
        let field = PropKey::Name(JsString::from_str(attribute));
        if !matches!(own_property(fields, &field), Some(Value::Bool(true))) {
            return Err(
                Bail::Unsupported(&"a data descriptor without every attribute true").into(),
            );
        }
    }
    let value_key = PropKey::Name(JsString::from_str("value"));
    let redefines = crate::object::own_attributes(obj, &key)
        .is_some_and(|attrs| attrs != crate::value::Attrs::DATA);
    if own_property(fields, &value_key).is_none() || redefines {
        return Err(Bail::Unsupported(&"a definition other than a plain data write").into());
    }
    let value = host.get_property(descriptor, &value_key)?;
    write_property(host, obj, key, value)?;
    Ok(())
}

#[inline(never)]
fn descriptor_flag(fields: &ObjRef<'_>, name: &str) -> EvalResult<Option<bool>> {
    let found = crate::object::lookup_property(fields, &PropKey::Name(JsString::from_str(name)))?;
    Ok(found.map(|value| crate::ops::to_boolean(&value)))
}

fn read_descriptor<'a>(
    host: &mut dyn Host<'a>,
    descriptor: &Value<'a>,
) -> Eval<'a, Descriptor<'a>> {
    let mut flags = [None; 4];
    if let Some(fields) = descriptor.as_object() {
        for (slot, name) in
            flags
                .iter_mut()
                .zip(["enumerable", "configurable", "value", "writable"])
        {
            *slot = descriptor_flag(fields, name)?;
        }
    }
    let value = host.get_property(descriptor, &PropKey::Name(JsString::from_str("value")))?;
    let [enumerable, configurable, present, writable] = flags;
    Ok(Descriptor {
        value: present.map(|_| value),
        writable,
        enumerable,
        configurable,
    })
}

fn native_get_prototype_of<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    host.get_prototype_of(&argument(args, 0))
}

fn native_has_own_property<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    crate::state::note_key_observation(&this);
    let key = string_key(to_js_string(&argument(args, 0), host)?);
    let Some(obj) = this.as_object() else {
        return primitive_has_own(host, &this, &key);
    };
    let own = own_property(obj, &key)
        .or_else(|| crate::object::has_pending_prototype(obj, &key).then_some(Value::Undefined));
    if own.is_none() && crate::object::decoding() && crate::object::is_arguments(obj) {
        crate::interp::refuse_arguments_miss(&key, None)?;
    }
    crate::object::refuse_undefined_dense_index(obj, &key, own.as_ref())?;
    if own.is_none() {
        crate::unmodelled::refuse_unmodelled_own_miss(&this, &key)?;
        host.refuse_namespace(&this)?;
    }
    Ok(Value::Bool(own.is_some()))
}

fn primitive_has_own<'a>(
    host: &mut dyn Host<'a>,
    this: &Value<'a>,
    key: &PropKey,
) -> Eval<'a, Value<'a>> {
    match this {
        Value::Undefined | Value::Null => Err(host.throw(
            ErrorKind::TypeError,
            "cannot convert undefined or null to object",
        )),
        Value::Str(text) => Ok(Value::Bool(
            crate::object::string_property(text, key).is_some(),
        )),
        _ => Ok(Value::Bool(false)),
    }
}

fn native_object_to_string<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if crate::object::decoding() && this.as_object().is_some() {
        let marker = PropKey::Name(JsString::from_str("@@toStringTag"));
        host.get_property(&this, &marker)?;
    }
    host.refuse_namespace(&this)?;
    let tag = match &this {
        Value::Undefined => "Undefined",
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(_) => "Number",
        Value::Str(_) => "String",
        Value::Object(obj) => object_tag(obj)?,
    };
    Ok(Value::Str(JsString::from_str(&format!("[object {tag}]"))))
}

fn object_tag<'a>(obj: &ObjRef<'a>) -> Eval<'a, &'static str> {
    let mut current = obj.clone();
    for _ in 0..crate::budget::BOUND_CHAIN_CAP {
        let target = match &current.borrow().kind {
            ObjKind::Array(_) => return Ok("Array"),
            ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => return Ok("Arguments"),
            ObjKind::Function(closure) => return Ok(closure.class_tag()),
            ObjKind::Native(_, _) => return Ok("Function"),
            ObjKind::Error(_) => return Ok("Error"),
            ObjKind::RegExp(_) => return Ok("RegExp"),
            ObjKind::Plain(_) => return Ok("Object"),
            ObjKind::Bound(bound) => bound.target.as_object().cloned(),
        };
        let Some(target) = target else {
            return Ok("Function");
        };
        current = target;
    }
    Err(Bail::ChainTooDeep(&"bound function").into())
}

fn native_value_of<'a>(
    _host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(this)
}
