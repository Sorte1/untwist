use untwist::{Options, Pipeline};
use untwist_deobfuscate::object_props;

const EXPECTED_INLINE_OBJECT_PROPS_NOTES: usize = 4;

fn game_options() -> Options {
    Options {
        unpack: false,
        jsx: false,
        unsafe_transforms: true,
        ..Options::default()
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-deobfuscate --test gltf_constant_maps"
)]
fn the_two_gltf_constant_maps_are_kept_untouched() {
    let Some(source) = untwist_testing::real_world_fixture("game.js") else {
        return;
    };
    let out = Pipeline::new(game_options()).run(&source);
    assert!(
        out.code().contains("FLOAT_MAT4: 35676"),
        "the WEBGL_CONSTANTS map moved or was inlined; re-measure and repin its substring"
    );
    for key in ["9728:", "9729:", "9984:", "9985:", "9986:", "9987:"] {
        assert!(
            out.code().contains(key),
            "the WEBGL_FILTERS map moved or was inlined; key {key} is no longer printed as-is"
        );
    }
    let inline_object_props_notes = out
        .notes
        .iter()
        .filter(|note| note.pass == object_props::PASS)
        .count();
    assert_eq!(
        inline_object_props_notes, EXPECTED_INLINE_OBJECT_PROPS_NOTES,
        "the inline-object-props note count moved; the two GLTF maps are never a readonly \
         candidate for this pass (their bindings carry opaque, unresolved uses from computed \
         member access with a non-literal key, so is_readonly_object refuses them before any \
         substitution or note is attempted) and contribute none of these notes. Four belong to \
         the unrelated prm/TWOFA objects, each reported once after notes are deduplicated. \
         The prop-hunt and Zombies mode objects set teams or lives in the literal and again in \
         their _defineProperty chain; the fold keeps one key \
         (both values are side-effect-free), so they add no ambiguous-key note; \
         re-measure before changing the constant"
    );
}

#[test]
fn a_map_read_through_a_literal_and_a_non_literal_key_is_left_alone() {
    let source = "\
function hi(k) {
  var MAP = { LINEAR: 9729, NEAREST: 9728 };
  console.log(MAP.LINEAR, MAP[k]);
}";
    let out = Pipeline::new(game_options()).run(source);
    assert!(
        out.code().contains("MAP.LINEAR"),
        "the static literal-key read was inlined even though the same binding also has a \
         non-literal computed read: {}",
        out.code()
    );
    assert!(
        out.code().contains("MAP[k]"),
        "the computed read itself was rewritten, not just the declaration: {}",
        out.code()
    );
}

#[test]
fn a_descriptor_array_reader_does_not_block_the_inline() {
    let source = "\
function hi() {
  var MAP = {};
  MAP.LINEAR = 9729;
  MAP.NEAREST = 9728;
  function Loader() {}
  _createClass(Loader, [{ key: \"filter\", value: function () { return MAP.LINEAR; } }]);
  return Loader;
}";
    let out = Pipeline::new(game_options()).run(source);
    assert!(
        !out.code().contains("MAP.LINEAR"),
        "the descriptor-array shape is refusing the inline again; the points-to story may be a \
         gate on this tree after all: {}",
        out.code()
    );
    assert!(
        out.code().contains("9729"),
        "the descriptor-array shape no longer inlines the map's value at all: {}",
        out.code()
    );
}
