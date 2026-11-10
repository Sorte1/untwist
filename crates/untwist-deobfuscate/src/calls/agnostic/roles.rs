use super::AKey;

pub(super) const DEFINE_KEYED: [&str; 2] = ["defineProperty", "deleteProperty"];

pub(super) const BATCH: [&str; 2] = ["defineProperties", "assign"];

pub(super) const GETTERS: [&str; 2] = ["__defineGetter__", "__defineSetter__"];

pub(super) const MUTATORS: [&str; 9] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "fill",
    "copyWithin",
    "reverse",
    "sort",
];

pub(super) const READERS: [&str; 5] = [
    "get",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "__lookupGetter__",
    "__lookupSetter__",
];

pub(super) const TARGETED_READERS: [&str; 2] = ["get", "getOwnPropertyDescriptor"];

pub(super) const RUNNERS: [&str; 14] = [
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "execScript",
    "require",
    "runInThisContext",
    "runInContext",
    "runInNewContext",
    "compileFunction",
    "Script",
    "SourceTextModule",
    "_compile",
];

pub(super) const SELECTOR_TAKERS: [&str; 6] = [
    "querySelector",
    "querySelectorAll",
    "closest",
    "matches",
    "webkitMatchesSelector",
    "msMatchesSelector",
];

pub(super) const CONSTRUCTS: [&str; 5] =
    ["construct", "revocable", "Proxy", "fromEntries", "parse"];

pub(super) const KEY_DEPTH: u8 = 32;

pub(super) const IDENTITY: &str = "identity";

pub(super) const CODE_HINTS: [&str; 5] = ["=", "(", "++", "--", "delete"];

pub(super) const FRESH_NATIVES: [&str; 2] = ["bind", "create"];

pub(super) const FRESH_REFUTERS: [&str; 3] = ["bind", "create", "get"];

pub(super) const INDUCTION_REFUTERS: [&str; 6] = [
    "Object",
    "defineProperty",
    "defineProperties",
    "__defineGetter__",
    "__defineSetter__",
    "deleteProperty",
];

pub(super) const ASSIGN_REFUTERS: [&str; 2] = ["assign", "Object"];

pub(super) fn is_definer_key(key: &str) -> bool {
    DEFINE_KEYED.contains(&key)
        || BATCH.contains(&key)
        || GETTERS.contains(&key)
        || MUTATORS.contains(&key)
        || key == "set"
        || key == "setPrototypeOf"
}

#[derive(Clone, Copy, Default)]
pub(super) struct Origin {
    pub(super) definer: bool,
    pub(super) runner: bool,
    pub(super) bind: bool,
    pub(super) apply: bool,
    pub(super) construct: bool,
}

impl Origin {
    pub(super) fn any(self) -> bool {
        self.definer || self.runner || self.bind || self.apply || self.construct
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyRoles {
    pub definer: bool,
    pub reader: bool,
    pub mutator: bool,
    pub runner: bool,
    pub construct: bool,
    pub forwards: bool,
    pub accessor_definer: bool,
}

pub fn key_roles(name: &str) -> KeyRoles {
    KeyRoles {
        definer: is_definer_key(name),
        reader: READERS.contains(&name),
        mutator: MUTATORS.contains(&name),
        runner: name == "constructor" || RUNNERS.contains(&name),
        construct: CONSTRUCTS.contains(&name),
        forwards: matches!(name, "call" | "apply" | "bind"),
        accessor_definer: name == "__proto__",
    }
}

pub(super) fn origin_of(key: &AKey<'_>) -> Origin {
    match key {
        AKey::Named(name) => {
            let roles = key_roles(name);
            Origin {
                definer: roles.definer || roles.reader,
                runner: roles.runner,
                bind: name == "bind",
                apply: name == "apply" || name == "call",
                construct: roles.construct,
            }
        }
        AKey::Index => Origin::default(),
        AKey::Pending(_) | AKey::Filtered(_) | AKey::Unknown => Origin {
            definer: true,
            runner: true,
            bind: true,
            apply: true,
            construct: true,
        },
    }
}

pub(super) fn reflective_origin(key: &AKey<'_>) -> Origin {
    let origin = origin_of(key);
    let accessor = matches!(key, AKey::Named(name) if key_roles(name).accessor_definer);
    Origin {
        definer: origin.definer || accessor,
        ..origin
    }
}
