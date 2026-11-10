use untwist_core::host_globals::{
    is_host_global, is_host_global_property, HOST_GLOBALS, NODE_BUILTIN_MODULES, WINDOW_PROPERTIES,
};

#[test]
fn both_lists_are_sorted_and_disjoint() {
    assert!(HOST_GLOBALS.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(WINDOW_PROPERTIES.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(WINDOW_PROPERTIES.iter().all(|name| !is_host_global(name)));
    assert!(NODE_BUILTIN_MODULES
        .windows(2)
        .all(|pair| pair[0] < pair[1]));
    assert!(NODE_BUILTIN_MODULES.iter().all(|name| is_host_global(name)));
}

#[test]
fn node_module_names_are_not_global_properties() {
    for name in ["fs", "path", "util", "os"] {
        assert!(!is_host_global_property(name), "{name}");
    }
}

#[test]
fn window_members_and_event_handlers_are_host_properties() {
    for name in [
        "status", "location", "top", "name", "length", "scroll", "onload", "onclick",
    ] {
        assert!(is_host_global_property(name), "{name}");
    }
}

#[test]
fn plain_names_are_not_host_properties() {
    for name in ["a", "total", "on", "onLoad", "o1"] {
        assert!(!is_host_global_property(name), "{name}");
    }
}
