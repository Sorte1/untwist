use super::identify::Size;

mod array;
mod class;
mod destructure;
#[cfg(feature = "test-hooks")]
mod fired;
mod generator;
mod object;

#[cfg(feature = "test-hooks")]
pub use fired::{note, take, Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    ToConsumableArray,
    ArrayWithoutHoles,
    IterableToArray,
    UnsupportedIterableToArray,
    NonIterableSpread,
    ArrayLikeToArray,
    SpreadArray,
    Read,
    Assign,
    ObjectSpread2,
    ObjectSpreadEach,
    ObjectSpreadProps,
    OwnKeys,
    DefineProperty,
    DefinePropertyPlain,
    ToPropertyKey,
    ToPrimitive,
    Typeof,
    SpreadValues,
    SpreadProps,
    DefNormalProp,
    SlicedToArray,
    ArrayWithHoles,
    IterableToArrayLimit,
    NonIterableRest,
    ObjectWithoutProperties,
    ObjectWithoutPropertiesLoose,
    Rest,
    ObjRest,
    ClassCallCheck,
    CreateClass,
    CreateClassOpen,
    DefineProperties,
    DefinePropertiesPlain,
    Inherits,
    InheritsOpen,
    SetPrototypeOf,
    GetPrototypeOf,
    CallSuper,
    PossibleConstructorReturn,
    AssertThisInitialized,
    IsNativeReflectConstruct,
    PublicField,
    Extends,
    Generator,
    Values,
    Awaiter,
    Async,
    Alias(Alias),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Alias {
    DefineProperty,
    DefineProperties,
    GetOwnPropertyDescriptors,
    GetOwnPropertySymbols,
    HasOwnProperty,
    PropertyIsEnumerable,
}

pub const ALIASES: [(&str, Alias); 6] = [
    ("defineProperty", Alias::DefineProperty),
    ("defineProperties", Alias::DefineProperties),
    (
        "getOwnPropertyDescriptors",
        Alias::GetOwnPropertyDescriptors,
    ),
    ("getOwnPropertySymbols", Alias::GetOwnPropertySymbols),
    ("hasOwnProperty", Alias::HasOwnProperty),
    ("propertyIsEnumerable", Alias::PropertyIsEnumerable),
];

#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub kind: Kind,
    pub hash: u64,
    pub params: u8,
    pub statements: u8,
    pub lead: u8,
    pub focus: u8,
    pub external: &'static [Kind],
    pub fixture: &'static str,
    pub cap: u32,
}

pub const FAMILIES: [&[Variant]; 5] = [
    array::VARIANTS,
    object::VARIANTS,
    destructure::VARIANTS,
    class::VARIANTS,
    generator::VARIANTS,
];

pub fn variants() -> impl Iterator<Item = &'static Variant> {
    FAMILIES.iter().flat_map(|family| family.iter())
}

pub const TOKEN_CAP: u32 = 176;

fn sized(variant: &Variant, size: Size) -> bool {
    variant.params == size.params
        && variant.statements == size.statements
        && variant.lead == size.lead
        && variant.focus == size.focus
}

const MASKED_PARAMS: usize = 8;

const fn size_masks() -> [u16; MASKED_PARAMS] {
    let mut masks = [0u16; MASKED_PARAMS];
    let mut family = 0;
    while family < FAMILIES.len() {
        let variants = FAMILIES[family];
        let mut at = 0;
        while at < variants.len() {
            let variant = &variants[at];
            if (variant.params as usize) < MASKED_PARAMS && variant.statements < 16 {
                masks[variant.params as usize] |= 1 << variant.statements;
            }
            at += 1;
        }
        family += 1;
    }
    masks
}

const SIZE_MASKS: [u16; MASKED_PARAMS] = size_masks();

pub fn sized_cap(size: Size) -> Option<u32> {
    let sized_like_one = size.statements < 16
        && SIZE_MASKS
            .get(usize::from(size.params))
            .is_some_and(|mask| mask & (1 << size.statements) != 0);
    if !sized_like_one {
        return None;
    }
    variants()
        .filter(|variant| sized(variant, size))
        .map(|variant| variant.cap)
        .max()
}

pub fn known(hash: u64, size: Size) -> Option<&'static Variant> {
    variants().find(|variant| variant.hash == hash && sized(variant, size))
}

pub const HELPER_NAMES: [(&str, Kind); 62] = [
    ("_toConsumableArray", Kind::ToConsumableArray),
    ("_to_consumable_array", Kind::ToConsumableArray),
    ("_arrayWithoutHoles", Kind::ArrayWithoutHoles),
    ("_array_without_holes", Kind::ArrayWithoutHoles),
    ("_iterableToArray", Kind::IterableToArray),
    ("_iterable_to_array", Kind::IterableToArray),
    (
        "_unsupportedIterableToArray",
        Kind::UnsupportedIterableToArray,
    ),
    (
        "_unsupported_iterable_to_array",
        Kind::UnsupportedIterableToArray,
    ),
    ("_arrayLikeToArray", Kind::ArrayLikeToArray),
    ("_array_like_to_array", Kind::ArrayLikeToArray),
    ("__spreadArray", Kind::SpreadArray),
    ("__read", Kind::Read),
    ("__assign", Kind::Assign),
    ("_objectSpread2", Kind::ObjectSpread2),
    ("_objectSpread", Kind::ObjectSpread2),
    ("_object_spread", Kind::ObjectSpreadEach),
    ("_object_spread_props", Kind::ObjectSpreadProps),
    ("_defineProperty", Kind::DefineProperty),
    ("_define_property", Kind::DefinePropertyPlain),
    ("__spreadValues", Kind::SpreadValues),
    ("__spreadProps", Kind::SpreadProps),
    ("__defNormalProp", Kind::DefNormalProp),
    ("_slicedToArray", Kind::SlicedToArray),
    ("_sliced_to_array", Kind::SlicedToArray),
    ("_arrayWithHoles", Kind::ArrayWithHoles),
    ("_array_with_holes", Kind::ArrayWithHoles),
    ("_iterableToArrayLimit", Kind::IterableToArrayLimit),
    ("_iterable_to_array_limit", Kind::IterableToArrayLimit),
    ("_nonIterableRest", Kind::NonIterableRest),
    ("_non_iterable_rest", Kind::NonIterableRest),
    ("_objectWithoutProperties", Kind::ObjectWithoutProperties),
    ("_object_without_properties", Kind::ObjectWithoutProperties),
    (
        "_objectWithoutPropertiesLoose",
        Kind::ObjectWithoutPropertiesLoose,
    ),
    (
        "_object_without_properties_loose",
        Kind::ObjectWithoutPropertiesLoose,
    ),
    ("__rest", Kind::Rest),
    ("__objRest", Kind::ObjRest),
    ("_classCallCheck", Kind::ClassCallCheck),
    ("_class_call_check", Kind::ClassCallCheck),
    ("_createClass", Kind::CreateClass),
    ("_create_class", Kind::CreateClassOpen),
    ("_defineProperties", Kind::DefineProperties),
    ("_inherits", Kind::Inherits),
    ("_setPrototypeOf", Kind::SetPrototypeOf),
    ("_set_prototype_of", Kind::SetPrototypeOf),
    ("_getPrototypeOf", Kind::GetPrototypeOf),
    ("_get_prototype_of", Kind::GetPrototypeOf),
    ("_callSuper", Kind::CallSuper),
    ("_call_super", Kind::CallSuper),
    (
        "_possibleConstructorReturn",
        Kind::PossibleConstructorReturn,
    ),
    (
        "_possible_constructor_return",
        Kind::PossibleConstructorReturn,
    ),
    ("_assertThisInitialized", Kind::AssertThisInitialized),
    ("_assert_this_initialized", Kind::AssertThisInitialized),
    ("_isNativeReflectConstruct", Kind::IsNativeReflectConstruct),
    (
        "_is_native_reflect_construct",
        Kind::IsNativeReflectConstruct,
    ),
    ("__publicField", Kind::PublicField),
    ("__extends", Kind::Extends),
    ("__generator", Kind::Generator),
    ("_ts_generator", Kind::Generator),
    ("__values", Kind::Values),
    ("_ts_values", Kind::Values),
    ("__awaiter", Kind::Awaiter),
    ("__async", Kind::Async),
];

pub fn is_consumer(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::ToConsumableArray
            | Kind::SpreadArray
            | Kind::Assign
            | Kind::ObjectSpread2
            | Kind::ObjectSpreadEach
            | Kind::ObjectSpreadProps
            | Kind::SpreadValues
            | Kind::SpreadProps
    )
}

pub fn is_destructurer(kind: Kind) -> bool {
    matches!(kind, Kind::SlicedToArray | Kind::Read)
}

pub fn is_class_trigger(kind: Kind) -> bool {
    matches!(kind, Kind::CreateClass | Kind::CreateClassOpen)
}

pub fn is_field_definer(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::DefineProperty | Kind::DefinePropertyPlain | Kind::PublicField
    )
}

pub fn is_machine_runtime(kind: Kind) -> bool {
    matches!(kind, Kind::Generator)
}

pub fn is_async_runner(kind: Kind) -> bool {
    matches!(kind, Kind::Awaiter | Kind::Async)
}

pub fn is_object_rest(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::ObjectWithoutProperties | Kind::Rest | Kind::ObjRest
    )
}

pub fn named_kind(name: &str) -> Option<Kind> {
    HELPER_NAMES
        .iter()
        .find(|(helper, _)| *helper == name)
        .map(|(_, kind)| *kind)
}

#[inline]
pub fn fire(pass: &'static str, shape: &'static str) {
    #[cfg(feature = "test-hooks")]
    fired::note(pass, shape);
    #[cfg(not(feature = "test-hooks"))]
    let _ = (pass, shape);
}
