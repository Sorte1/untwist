use untwist_core::semantic::builtins::Reliance;

pub use untwist_core::semantic::builtins::{intact, scan, Builtins};

pub const RELIED_ON: Reliance = Reliance {
    roots: &["Object", "Array", "Function", "Symbol", "String"],
    members: &[
        (
            "Object",
            false,
            &[
                "assign",
                "keys",
                "create",
                "defineProperty",
                "defineProperties",
                "getOwnPropertyDescriptor",
                "getOwnPropertyDescriptors",
                "getOwnPropertySymbols",
                "getPrototypeOf",
                "setPrototypeOf",
                "prototype",
            ],
        ),
        ("Array", false, &["isArray", "from", "of", "prototype"]),
        (
            "Array",
            true,
            &[
                "concat",
                "slice",
                "push",
                "filter",
                "forEach",
                "indexOf",
                "join",
                "values",
                "keys",
                "entries",
                "constructor",
            ],
        ),
        ("String", false, &["prototype"]),
        ("String", true, &["slice", "constructor"]),
        ("Function", false, &["prototype"]),
        ("Function", true, &["apply", "call", "bind", "constructor"]),
    ],
};
