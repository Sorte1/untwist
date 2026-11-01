use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::stamp::{
    decoder_index, decoder_name, find_stamped, string_array_name, string_array_unit, IndexAllocator,
};

const STAMPED: &str = "\
function __STRING_ARRAY__() { var h = ['log']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
var __DECODE_1__ = function (c, d) { var e = __STRING_ARRAY__(); return e[c]; };
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); return e[c]; }
function untouched() {}";

#[test]
fn names_round_trip() {
    assert_eq!(string_array_name(0), "__STRING_ARRAY__");
    assert_eq!(string_array_name(3), "__STRING_ARRAY_3__");
    assert_eq!(string_array_unit("__STRING_ARRAY__"), Some(0));
    assert_eq!(string_array_unit("__STRING_ARRAY_3__"), Some(3));
    assert_eq!(string_array_unit("__DECODE_0__"), None);
    assert_eq!(decoder_name(7), "__DECODE_7__");
    assert_eq!(decoder_index("__DECODE_7__"), Some(7));
    assert_eq!(decoder_index("__STRING_ARRAY__"), None);
}

#[test]
fn stamped_declarations_are_indexed_by_their_number_not_by_source_order() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, STAMPED);
    let stamped = find_stamped(&parsed.program);
    assert_eq!(stamped.arrays.len(), 1);
    assert_eq!(stamped.arrays[0].name, "__STRING_ARRAY__");
    assert_eq!(stamped.decoders.len(), 2);
    assert_eq!(stamped.decoders[0].name, "__DECODE_0__");
    assert_eq!(stamped.decoders[1].name, "__DECODE_1__");
    assert!(stamped.decoders[0].statement.start > stamped.decoders[1].statement.start);
}

#[test]
fn an_unstamped_program_yields_nothing() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function a() {}\nvar b = function () {};");
    assert!(find_stamped(&parsed.program).is_empty());
}

#[test]
fn the_index_allocator_skips_every_number_already_bound() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, STAMPED);
    let sem = build_scoping(&parsed.program);

    let mut decoders = IndexAllocator::decoders(&sem.scoping);
    assert_eq!(decoders.allocate(), 2);
    assert_eq!(decoders.allocate(), 3);

    let mut arrays = IndexAllocator::string_arrays(&sem.scoping);
    assert_eq!(arrays.allocate(), 1);
}

#[test]
fn the_index_allocator_starts_at_zero_on_an_unstamped_program() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function a() {}\nvar b = function () {};");
    let sem = build_scoping(&parsed.program);
    let mut decoders = IndexAllocator::decoders(&sem.scoping);
    assert_eq!(decoders.allocate(), 0);
    assert_eq!(decoders.allocate(), 1);
}

#[test]
fn wrappers_find_the_decoders_stamped_earlier_in_the_same_traversal() {
    let alloc = Allocator::default();
    let source = "function arr() { var h = ['log']; arr = function () { return h; }; return arr(); }\nfunction b(c, d) { var e = arr(); return e[c]; }\nconst alias = b;\nconsole.log(alias(0));";
    let mut program = parse_program(&alloc, source).program;
    let sem = build_scoping(&program);
    let mut group = untwist_core::pass::PassGroup::from_ctors(
        &[
            untwist_deobfuscate::string_array::boxed,
            untwist_deobfuscate::decoder::boxed,
            untwist_deobfuscate::wrappers::boxed,
        ],
        untwist_core::pass::PassOptions::default(),
    );
    untwist_core::pass::run_once(&alloc, &mut program, sem, &mut group);
    let code = untwist_core::print::print(&program, untwist_core::print::PrintStyle::Pretty);
    assert!(code.contains("console.log(__DECODE_0__(0));"), "{code}");
}
