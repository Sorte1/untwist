use untwist_core::pass::{ExpressionKind, StatementKind};

use super::{Kind, Variant, TOKEN_CAP};

const BABEL_7_29_SAFARI9_IIFE_TERSER_CLASS_EARLY_RETURN: &str =
    "fixtures/untranspile/matrix/babel-7.29/safari9-iife-terser/class-early-return.js";
const BABEL_7_12_IE11_FOR_OF: &str = "fixtures/untranspile/matrix/babel-7.12/ie11/for-of.js";
const BABEL_7_12_IE11_RUNTIME_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11-runtime/for-of.js";
const BABEL_7_20_IE11_FOR_OF: &str = "fixtures/untranspile/matrix/babel-7.20/ie11/for-of.js";
const BABEL_7_20_IE11_RUNTIME_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-7.20/ie11-runtime/for-of.js";
const BABEL_7_24_IE11_FOR_OF: &str = "fixtures/untranspile/matrix/babel-7.24/ie11/for-of.js";
const BABEL_7_24_IE11_RUNTIME_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-7.24/ie11-runtime/for-of.js";
const BABEL_7_29_IE11_TERSER_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11-terser/for-of.js";
const BABEL_7_28_IE11_RUNTIME_IIFE_TERSER_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-7.28/ie11-runtime-iife-terser/for-of.js";
const BABEL_7_29_IE11_ROLLUP_TERSER_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11-rollup-terser/for-of.js";
const BABEL_7_28_IE11_GENERATORS: &str =
    "fixtures/untranspile/matrix/babel-7.28/ie11/generators.js";
const BABEL_7_28_IE11_TERSER_GENERATORS: &str =
    "fixtures/untranspile/matrix/babel-7.28/ie11-terser/generators.js";
const BABEL_7_29_IE11_GENERATORS: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11/generators.js";
const BABEL_7_29_IE11_TERSER_GENERATORS: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11-terser/generators.js";
const BABEL_7_29_IE11_ASYNC_AWAIT: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11/async-await.js";
const BABEL_8_IE11_FOR_OF: &str = "fixtures/untranspile/matrix/babel-8/ie11/for-of.js";
const BABEL_8_IE11_RUNTIME_FOR_OF: &str =
    "fixtures/untranspile/matrix/babel-8/ie11-runtime/for-of.js";
const BABEL_7_0_IE11_ARROW_THIS: &str = "fixtures/untranspile/matrix/babel-7.0/ie11/arrow-this.js";
const BABEL_7_0_IE11_DESTRUCTURING: &str =
    "fixtures/untranspile/matrix/babel-7.0/ie11/destructuring.js";
const BABEL_7_0_IE11_GENERATORS: &str = "fixtures/untranspile/matrix/babel-7.0/ie11/generators.js";
const BABEL_7_0_IE11_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.0/ie11/object-spread-rest.js";
const BABEL_7_12_IE11_LOOSE_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11-loose/object-spread-rest.js";
const BABEL_7_12_IE11_RUNTIME_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11-runtime/object-spread-rest.js";
const BABEL_7_12_IE11_ARROW_THIS: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11/arrow-this.js";
const BABEL_7_12_IE11_BLOCK_SCOPING: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11/block-scoping.js";
const BABEL_7_12_IE11_CLASS_BUILTINS: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11/class-builtins.js";
const BABEL_7_12_IE11_DESTRUCTURING: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11/destructuring.js";
const BABEL_7_12_IE11_GENERATORS: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11/generators.js";
const BABEL_7_12_IE11_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.12/ie11/object-spread-rest.js";
const BABEL_7_20_IE11_RUNTIME_CLASS_FIELDS: &str =
    "fixtures/untranspile/matrix/babel-7.20/ie11-runtime/class-fields.js";
const BABEL_7_20_IE11_CLASS_BUILTINS: &str =
    "fixtures/untranspile/matrix/babel-7.20/ie11/class-builtins.js";
const BABEL_7_20_IE11_CLASS_FIELDS: &str =
    "fixtures/untranspile/matrix/babel-7.20/ie11/class-fields.js";
const BABEL_7_24_IE11_LOOSE_CLASS_BASIC: &str =
    "fixtures/untranspile/matrix/babel-7.24/ie11-loose/class-basic.js";
const BABEL_7_24_IE11_RUNTIME_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.24/ie11-runtime/object-spread-rest.js";
const BABEL_7_29_IE11_LOOSE_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11-loose/object-spread-rest.js";
const BABEL_7_29_IE11_RUNTIME_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11-runtime/object-spread-rest.js";
const BABEL_7_29_IE11_ARROW_THIS: &str =
    "fixtures/untranspile/matrix/babel-7.29/ie11/arrow-this.js";
const BABEL_8_IE11_DESTRUCTURING: &str =
    "fixtures/untranspile/matrix/babel-8/ie11/destructuring.js";
const BABEL_8_IE11_GENERATORS: &str = "fixtures/untranspile/matrix/babel-8/ie11/generators.js";
const BABEL6_6_IE11_CLASS_BUILTINS: &str =
    "fixtures/untranspile/matrix/babel6-6/ie11/class-builtins.js";
const BABEL6_6_IE11_FOR_OF: &str = "fixtures/untranspile/matrix/babel6-6/ie11/for-of.js";
const ESBUILD_0_14_ES2015_CLASS_FIELDS: &str =
    "fixtures/untranspile/matrix/esbuild-0.14/es2015/class-fields.js";
const SWC_1_16_ES5_TERSER_ARROW_THIS: &str =
    "fixtures/untranspile/matrix/swc-1.16/es5-terser/arrow-this.js";
const SWC_1_16_ES5_TERSER_ASYNC_AWAIT: &str =
    "fixtures/untranspile/matrix/swc-1.16/es5-terser/async-await.js";
const SWC_1_16_ES5_TERSER_CLASS_BUILTINS: &str =
    "fixtures/untranspile/matrix/swc-1.16/es5-terser/class-builtins.js";
const SWC_1_16_ES5_TERSER_DESTRUCTURING: &str =
    "fixtures/untranspile/matrix/swc-1.16/es5-terser/destructuring.js";
const SWC_1_16_ES5_TERSER_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/swc-1.16/es5-terser/object-spread-rest.js";
const SWC_1_2_ES5_ARROW_THIS: &str = "fixtures/untranspile/matrix/swc-1.2/es5/arrow-this.js";
const SWC_1_2_ES5_DESTRUCTURING: &str = "fixtures/untranspile/matrix/swc-1.2/es5/destructuring.js";
const SWC_1_2_ES5_GENERATORS: &str = "fixtures/untranspile/matrix/swc-1.2/es5/generators.js";
const SWC_1_3_ES5_ASYNC_AWAIT: &str = "fixtures/untranspile/matrix/swc-1.3/es5/async-await.js";
const TS_3_9_ES3_ASYNC_AWAIT: &str = "fixtures/untranspile/matrix/ts-3.9/es3/async-await.js";
const TS_3_9_ES5_TSLIB_ASYNC_AWAIT: &str =
    "fixtures/untranspile/matrix/ts-3.9/es5-tslib/async-await.js";
const TS_3_9_ES5_TSLIB_DESTRUCTURING: &str =
    "fixtures/untranspile/matrix/ts-3.9/es5-tslib/destructuring.js";
const TS_3_9_ES5_TSLIB_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/ts-3.9/es5-tslib/object-spread-rest.js";
const TS_4_9_ES3_ASYNC_AWAIT: &str = "fixtures/untranspile/matrix/ts-4.9/es3/async-await.js";
const TS_4_9_ES5_TSLIB_GENERATORS: &str =
    "fixtures/untranspile/matrix/ts-4.9/es5-tslib/generators.js";
const TS_5_9_ES5_TERSER_ASYNC_AWAIT: &str =
    "fixtures/untranspile/matrix/ts-5.9/es5-terser/async-await.js";
const TS_5_9_ES5_TERSER_CLASS_BUILTINS: &str =
    "fixtures/untranspile/matrix/ts-5.9/es5-terser/class-builtins.js";
const TS_5_9_ES5_TERSER_GENERATORS: &str =
    "fixtures/untranspile/matrix/ts-5.9/es5-terser/generators.js";
const TS_5_9_ES5_TERSER_OBJECT_SPREAD_REST: &str =
    "fixtures/untranspile/matrix/ts-5.9/es5-terser/object-spread-rest.js";

const EXPRESSION_STATEMENT: u8 = StatementKind::ExpressionStatement as u8;
const FOR_STATEMENT: u8 = StatementKind::ForStatement as u8;
const IF_STATEMENT: u8 = StatementKind::IfStatement as u8;
const RETURN_STATEMENT: u8 = StatementKind::ReturnStatement as u8;
const THROW_STATEMENT: u8 = StatementKind::ThrowStatement as u8;
const VARIABLE_DECLARATION: u8 = StatementKind::VariableDeclaration as u8;
const FUNCTION_DECLARATION: u8 = StatementKind::FunctionDeclaration as u8;
const IDENTIFIER_FOCUS: u8 = ExpressionKind::Identifier as u8;
const ARRAY_EXPRESSION_FOCUS: u8 = ExpressionKind::ArrayExpression as u8;
const ASSIGNMENT_EXPRESSION_FOCUS: u8 = ExpressionKind::AssignmentExpression as u8;
const CALL_EXPRESSION_FOCUS: u8 = ExpressionKind::CallExpression as u8;
const CONDITIONAL_EXPRESSION_FOCUS: u8 = ExpressionKind::ConditionalExpression as u8;
const FUNCTION_EXPRESSION_FOCUS: u8 = ExpressionKind::FunctionExpression as u8;
const LOGICAL_EXPRESSION_FOCUS: u8 = ExpressionKind::LogicalExpression as u8;
const NEW_EXPRESSION_FOCUS: u8 = ExpressionKind::NewExpression as u8;
const OBJECT_EXPRESSION_FOCUS: u8 = ExpressionKind::ObjectExpression as u8;
const UNARY_EXPRESSION_FOCUS: u8 = ExpressionKind::UnaryExpression as u8;
const DECLARED_FOCUS: u8 = u8::MAX - 1;
const NO_FOCUS: u8 = u8::MAX;
const ARROW_OR_FUNCTION_FOCUS: u8 = ExpressionKind::FunctionExpression as u8;
const TRY_STATEMENT: u8 = StatementKind::TryStatement as u8;
const STATIC_MEMBER_FOCUS: u8 = ExpressionKind::StaticMemberExpression as u8;

const fn variant(
    kind: Kind,
    hash: u128,
    (params, statements): (u8, u8),
    (lead, focus): (u8, u8),
    (external, tokens): (&'static [Kind], u32),
    fixture: &'static str,
) -> Variant {
    Variant {
        kind,
        hash,
        params,
        statements,
        lead,
        focus,
        external,
        fixture,
        cap: if tokens > TOKEN_CAP {
            tokens
        } else {
            TOKEN_CAP
        },
    }
}

pub const VARIANTS: &[Variant] = &[
    variant(
        Kind::ArrayWithoutHoles,
        0xc4bb_3435_78cc_3ee3_3dd7_4e5b_5615_429e,
        (1, 1),
        (IF_STATEMENT, CALL_EXPRESSION_FOCUS),
        (&[], 43),
        BABEL_7_0_IE11_GENERATORS,
    ),
    variant(
        Kind::Assign,
        0xdcd0_ae23_0c4f_5e76_433c_1617_838e_8a65,
        (0, 2),
        (EXPRESSION_STATEMENT, ASSIGNMENT_EXPRESSION_FOCUS),
        (&[], 76),
        TS_3_9_ES5_TSLIB_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::Assign,
        0x68d4_9e33_c4a7_b04b_9e0c_024c_d321_11d0,
        (0, 2),
        (EXPRESSION_STATEMENT, ASSIGNMENT_EXPRESSION_FOCUS),
        (&[], 79),
        TS_5_9_ES5_TERSER_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::Awaiter,
        0x3a05_82e4_6805_5020_9d3d_6d7c_ec38_c4c8,
        (4, 2),
        (FUNCTION_DECLARATION, NO_FOCUS),
        (&[], 143),
        TS_3_9_ES5_TSLIB_ASYNC_AWAIT,
    ),
    variant(
        Kind::Awaiter,
        0xe6e3_f361_5b4f_24f6_212e_c51f_1289_50cb,
        (4, 1),
        (RETURN_STATEMENT, NEW_EXPRESSION_FOCUS),
        (&[], 142),
        TS_5_9_ES5_TERSER_ASYNC_AWAIT,
    ),
    variant(
        Kind::CreateClassOpen,
        0x22e8_5aef_8622_6fef_a826_063b_2214_a05f,
        (0, 2),
        (FUNCTION_DECLARATION, NO_FOCUS),
        (&[], 98),
        BABEL6_6_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::DefineProperty,
        0x34c7_ea94_11dc_790c_0858_864a_b7d6_1e6f,
        (3, 3),
        (EXPRESSION_STATEMENT, ASSIGNMENT_EXPRESSION_FOCUS),
        (&[Kind::ToPropertyKey], 50),
        BABEL_7_20_IE11_CLASS_FIELDS,
    ),
    variant(
        Kind::Extends,
        0xf159_7576_f53d_1880_e99a_e39b_95f7_408d,
        (0, 2),
        (VARIABLE_DECLARATION, FUNCTION_EXPRESSION_FOCUS),
        (&[], 153),
        TS_5_9_ES5_TERSER_CLASS_BUILTINS,
    ),
    variant(
        Kind::Generator,
        0x7395_1ea1_9eb6_a868_a514_6788_ac54_7488,
        (2, 12),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[], 531),
        SWC_1_16_ES5_TERSER_ASYNC_AWAIT,
    ),
    variant(
        Kind::Generator,
        0x2c3e_0ed7_0851_904a_5f85_8796_73c8_ed17,
        (2, 10),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[], 483),
        SWC_1_3_ES5_ASYNC_AWAIT,
    ),
    variant(
        Kind::Generator,
        0x6c1c_81d2_915a_4dd7_bc71_c9a3_6f0c_d665,
        (2, 10),
        (VARIABLE_DECLARATION, OBJECT_EXPRESSION_FOCUS),
        (&[], 488),
        TS_3_9_ES3_ASYNC_AWAIT,
    ),
    variant(
        Kind::Generator,
        0xc290_a796_1ff3_aafd_4f7a_58d0_b19f_895b,
        (2, 12),
        (VARIABLE_DECLARATION, OBJECT_EXPRESSION_FOCUS),
        (&[], 515),
        TS_3_9_ES5_TSLIB_ASYNC_AWAIT,
    ),
    variant(
        Kind::Generator,
        0x5092_bbc7_8c31_e069_4d8e_27f6_b452_42e7,
        (2, 10),
        (VARIABLE_DECLARATION, OBJECT_EXPRESSION_FOCUS),
        (&[], 502),
        TS_4_9_ES3_ASYNC_AWAIT,
    ),
    variant(
        Kind::Generator,
        0x818b_8da0_187f_73e2_ce2f_e418_e44c_2ed4,
        (2, 11),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[], 518),
        TS_5_9_ES5_TERSER_ASYNC_AWAIT,
    ),
    variant(
        Kind::GetPrototypeOf,
        0xa19c_51bb_7828_93fb_f6d6_6dc5_8fa1_92e6,
        (1, 2),
        (EXPRESSION_STATEMENT, ASSIGNMENT_EXPRESSION_FOCUS),
        (&[], 39),
        BABEL_7_20_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::Inherits,
        0xe019_2601_f198_955d_c2e8_9f26_8b2a_ced3,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::SetPrototypeOf], 68),
        BABEL_7_20_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0xc196_61c8_852f_487f_002b_492f_db60_4fc5,
        (0, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 62),
        BABEL_7_12_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0x6e24_343f_d42e_42ae_22c0_17d0_0a42_4bd9,
        (0, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 62),
        BABEL_7_20_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::IterableToArray,
        0x7591_d593_9e7d_a9ff_6685_5913_a29a_53a4,
        (1, 1),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 33),
        BABEL_7_0_IE11_GENERATORS,
    ),
    variant(
        Kind::IterableToArray,
        0xb9ac_c466_3672_00ea_896c_a4f4_cc54_cc3a,
        (1, 1),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 26),
        BABEL_7_12_IE11_GENERATORS,
    ),
    variant(
        Kind::IterableToArray,
        0x59d4_96c9_c9ac_4a02_50e1_194b_f856_0dc6,
        (1, 3),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[], 94),
        BABEL_8_IE11_GENERATORS,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x7250_d9c3_2db3_59d2_ba17_212c_079e_15cf,
        (2, 6),
        (VARIABLE_DECLARATION, ARRAY_EXPRESSION_FOCUS),
        (&[], 109),
        BABEL_7_0_IE11_DESTRUCTURING,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x81cb_5ad6_ae06_0095_f0f6_cf30_96d2_24a5,
        (2, 7),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 125),
        BABEL_7_12_IE11_DESTRUCTURING,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0x9e26_b26c_d05d_f225_c257_d124_d34c_1d3c,
        (2, 3),
        (VARIABLE_DECLARATION, CONDITIONAL_EXPRESSION_FOCUS),
        (&[], 201),
        BABEL_8_IE11_DESTRUCTURING,
    ),
    variant(
        Kind::IterableToArrayLimit,
        0xcb48_e9ff_d5c4_693d_73ad_81c9_1a2f_eee7,
        (2, 2),
        (VARIABLE_DECLARATION, CONDITIONAL_EXPRESSION_FOCUS),
        (&[], 133),
        SWC_1_16_ES5_TERSER_DESTRUCTURING,
    ),
    variant(
        Kind::NonIterableRest,
        0xd744_e13f_98d2_9243_f00f_2710_3735_b827,
        (0, 1),
        (THROW_STATEMENT, NEW_EXPRESSION_FOCUS),
        (&[], 8),
        BABEL_7_0_IE11_DESTRUCTURING,
    ),
    variant(
        Kind::NonIterableRest,
        0xb63e_a04e_6baa_63c8_fbc8_9c1c_aef1_66c9,
        (0, 1),
        (THROW_STATEMENT, NEW_EXPRESSION_FOCUS),
        (&[], 8),
        SWC_1_2_ES5_DESTRUCTURING,
    ),
    variant(
        Kind::NonIterableSpread,
        0x6c84_3504_ad26_155d_2d21_b946_6ffc_7f27,
        (0, 1),
        (THROW_STATEMENT, NEW_EXPRESSION_FOCUS),
        (&[], 8),
        BABEL_7_0_IE11_GENERATORS,
    ),
    variant(
        Kind::NonIterableSpread,
        0x3269_8a1e_8625_62ef_0878_c6bd_4bc0_bc90,
        (0, 1),
        (THROW_STATEMENT, NEW_EXPRESSION_FOCUS),
        (&[], 8),
        SWC_1_2_ES5_GENERATORS,
    ),
    variant(
        Kind::ObjectSpread2,
        0xde22_4b8d_1edf_5b6e_cacf_9a79_1287_8c5f,
        (1, 2),
        (FOR_STATEMENT, DECLARED_FOCUS),
        (&[Kind::OwnKeys, Kind::DefinePropertyPlain], 106),
        BABEL_7_12_IE11_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ObjectSpreadEach,
        0xa2aa_6967_ec68_fcbd_210d_56dc_0640_dee8,
        (1, 2),
        (FOR_STATEMENT, DECLARED_FOCUS),
        (&[Kind::DefinePropertyPlain], 97),
        BABEL_7_0_IE11_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ObjectSpreadEach,
        0x6481_ba0e_8b16_24e8_1680_9ca9_8e1c_e397,
        (1, 2),
        (FOR_STATEMENT, DECLARED_FOCUS),
        (&[Kind::DefinePropertyPlain], 97),
        SWC_1_16_ES5_TERSER_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0xd8c0_c1a7_ccaf_31cf_0440_d7a8_97f1_fc32,
        (2, 3),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 51),
        BABEL_7_29_SAFARI9_IIFE_TERSER_CLASS_EARLY_RETURN,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x1aee_4423_6510_8d2c_378a_0268_bc76_7371,
        (2, 2),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof, Kind::AssertThisInitialized], 37),
        BABEL_7_20_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0xbf7c_fa41_94e5_3b84_1913_0195_49e2_58ee,
        (2, 1),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof, Kind::AssertThisInitialized], 30),
        SWC_1_16_ES5_TERSER_CLASS_BUILTINS,
    ),
    variant(
        Kind::PublicField,
        0xd083_76f9_3dd4_bcdf_c198_a793_56c1_537e,
        (3, 2),
        (EXPRESSION_STATEMENT, CALL_EXPRESSION_FOCUS),
        (&[Kind::DefNormalProp], 25),
        ESBUILD_0_14_ES2015_CLASS_FIELDS,
    ),
    variant(
        Kind::Read,
        0x75b9_46de_8db6_10fd_7471_2e5b_47d3_0dae,
        (2, 8),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[], 119),
        TS_3_9_ES5_TSLIB_DESTRUCTURING,
    ),
    variant(
        Kind::SetPrototypeOf,
        0x7b8e_00cf_9463_fc8d_6dc4_8f52_4450_a8a0,
        (2, 2),
        (EXPRESSION_STATEMENT, ASSIGNMENT_EXPRESSION_FOCUS),
        (&[], 42),
        BABEL_7_20_IE11_CLASS_BUILTINS,
    ),
    variant(
        Kind::SlicedToArray,
        0x1cb1_c57e_335e_f434_93f7_a38d_af08_8135,
        (2, 1),
        (RETURN_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (
            &[
                Kind::ArrayWithHoles,
                Kind::IterableToArrayLimit,
                Kind::NonIterableRest,
            ],
            20,
        ),
        BABEL_7_0_IE11_DESTRUCTURING,
    ),
    variant(
        Kind::SlicedToArray,
        0x501e_3d15_2132_98b9_8d3e_b44d_cf22_83ba,
        (2, 1),
        (RETURN_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (
            &[
                Kind::IterableToArrayLimit,
                Kind::UnsupportedIterableToArray,
                Kind::NonIterableRest,
            ],
            21,
        ),
        BABEL_8_IE11_DESTRUCTURING,
    ),
    variant(
        Kind::SlicedToArray,
        0x9cf1_1008_2ffe_4c2b_4d8e_df1b_920c_4b77,
        (0, 2),
        (FUNCTION_DECLARATION, NO_FOCUS),
        (&[], 147),
        BABEL6_6_IE11_FOR_OF,
    ),
    variant(
        Kind::SpreadArray,
        0x2085_4762_ac31_32e2_af40_3dcc_1480_05fd,
        (3, 2),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 87),
        TS_4_9_ES5_TSLIB_GENERATORS,
    ),
    variant(
        Kind::SpreadArray,
        0x1323_55fe_59f3_60d7_f7ad_72bb_5f2f_0a5f,
        (3, 2),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 91),
        TS_5_9_ES5_TERSER_GENERATORS,
    ),
    variant(
        Kind::ToConsumableArray,
        0x49ed_9851_c04e_61d2_3968_f1ad_d78f_115b,
        (1, 1),
        (RETURN_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (
            &[
                Kind::ArrayWithoutHoles,
                Kind::IterableToArray,
                Kind::NonIterableSpread,
            ],
            17,
        ),
        BABEL_7_0_IE11_GENERATORS,
    ),
    variant(
        Kind::ToPrimitive,
        0xdd27_33fa_f80a_55c7_e894_3a92_344d_e9f3,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 65),
        BABEL_7_12_IE11_LOOSE_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPrimitive,
        0xedc1_142e_2e15_c1e6_b6ee_0d24_ae40_f0f0,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof], 60),
        BABEL_7_12_IE11_RUNTIME_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPrimitive,
        0x1a5d_fe28_cfee_ad01_ec21_eeb8_46d0_14ea,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof], 67),
        BABEL_7_12_IE11_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPrimitive,
        0x466b_506e_d850_29d7_f9b3_1dcd_d622_d1c1,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof], 65),
        BABEL_7_20_IE11_RUNTIME_CLASS_FIELDS,
    ),
    variant(
        Kind::ToPrimitive,
        0xf402_4b47_9dfb_b2db_6a11_eb63_dfb8_b092,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 64),
        BABEL_7_24_IE11_LOOSE_CLASS_BASIC,
    ),
    variant(
        Kind::ToPrimitive,
        0x2d06_de73_8a22_020f_8ea0_f52c_97af_c8e4,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof], 59),
        BABEL_7_24_IE11_RUNTIME_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPrimitive,
        0xea61_10ea_0427_dcbb_8f2c_8f80_8344_65df,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 70),
        BABEL_7_29_IE11_LOOSE_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPrimitive,
        0x845e_ed7c_355d_9c51_97c2_a6cf_76ae_e1a5,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof], 65),
        BABEL_7_29_IE11_RUNTIME_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPrimitive,
        0x98f1_b5e7_a179_6051_b4ee_78de_d921_0f9f,
        (2, 4),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::Typeof], 72),
        BABEL_7_29_IE11_ARROW_THIS,
    ),
    variant(
        Kind::ToPropertyKey,
        0xa737_2040_203c_bafb_04a5_58b6_3c67_15bb,
        (1, 2),
        (VARIABLE_DECLARATION, CALL_EXPRESSION_FOCUS),
        (&[Kind::ToPrimitive], 26),
        BABEL_7_12_IE11_LOOSE_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPropertyKey,
        0x692e_05df_a792_d438_3197_a875_058e_0d0e,
        (1, 2),
        (VARIABLE_DECLARATION, CALL_EXPRESSION_FOCUS),
        (&[Kind::ToPrimitive, Kind::Typeof], 27),
        BABEL_7_12_IE11_OBJECT_SPREAD_REST,
    ),
    variant(
        Kind::ToPropertyKey,
        0x574d_acf0_4e38_0058_d3d9_f4fa_65c0_e4ad,
        (1, 2),
        (VARIABLE_DECLARATION, CALL_EXPRESSION_FOCUS),
        (&[Kind::ToPrimitive], 26),
        BABEL_7_24_IE11_LOOSE_CLASS_BASIC,
    ),
    variant(
        Kind::Typeof,
        0xb000_f00d_23b4_d212_af29_4ccc_a12e_dcdc,
        (1, 2),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 71),
        BABEL_7_0_IE11_ARROW_THIS,
    ),
    variant(
        Kind::Typeof,
        0x6d4a_4890_fc48_b6fd_54bc_120f_9a3e_4c8a,
        (1, 2),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 73),
        BABEL_7_12_IE11_ARROW_THIS,
    ),
    variant(
        Kind::Typeof,
        0x7dda_fe93_a69f_3825_55fc_327a_0cff_34ff,
        (1, 1),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 26),
        SWC_1_16_ES5_TERSER_ARROW_THIS,
    ),
    variant(
        Kind::Typeof,
        0x895f_e7fd_acde_e411_ccd0_4bf8_387e_a21f,
        (1, 1),
        (IF_STATEMENT, LOGICAL_EXPRESSION_FOCUS),
        (&[], 27),
        SWC_1_2_ES5_ARROW_THIS,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0x9b07_af2e_35a7_fbaa_32a7_cb03_9cb8_03e9,
        (2, 6),
        (IF_STATEMENT, UNARY_EXPRESSION_FOCUS),
        (&[Kind::ArrayLikeToArray], 90),
        BABEL_7_12_IE11_BLOCK_SCOPING,
    ),
    variant(
        Kind::UnsupportedIterableToArray,
        0xf1b6_31d2_e72b_9527_2c29_9ef5_a92c_2e80,
        (2, 1),
        (IF_STATEMENT, IDENTIFIER_FOCUS),
        (&[Kind::ArrayLikeToArray], 91),
        SWC_1_16_ES5_TERSER_DESTRUCTURING,
    ),
    variant(
        Kind::ForOfIterator,
        0x22d2_6ded_7f28_7957_b4a7_ae61_2cd3_9a89,
        (2, 6),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 212),
        BABEL_7_12_IE11_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0x8c1c_ef01_a0eb_e85e_38c8_7a34_ff0a_8aae,
        (2, 6),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 203),
        BABEL_7_12_IE11_RUNTIME_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0xa867_f61f_c41f_4b6e_dda2_28e8_6128_e106,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 215),
        BABEL_7_20_IE11_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0x8bb9_5c65_791c_51e3_4d61_750f_014e_d388,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 206),
        BABEL_7_20_IE11_RUNTIME_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0x66a6_27d8_fb6f_ae61_772d_66bd_6ebf_f5ba,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 216),
        BABEL_7_24_IE11_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0xb222_8b91_7096_5c0b_e8b4_75c0_e03d_352f,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 207),
        BABEL_7_24_IE11_RUNTIME_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0x141d_a36f_26b6_b924_19c4_fe2c_d346_478f,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 209),
        BABEL_7_29_IE11_TERSER_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0xacf7_0344_bf23_3e7a_47b6_43b3_5f6a_74aa,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::ArrayLikeToArray], 287),
        BABEL_7_28_IE11_RUNTIME_IIFE_TERSER_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0x2e6a_e86c_3904_0c14_cccf_8907_df8b_ad58,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 234),
        BABEL_8_IE11_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0xd982_3a52_4be2_33be_7952_1f7c_37a0_4bc7,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 225),
        BABEL_8_IE11_RUNTIME_FOR_OF,
    ),
    variant(
        Kind::ForOfIterator,
        0xb067_bdd7_1751_98a8_4496_5bdd_dae4_126a,
        (2, 6),
        (VARIABLE_DECLARATION, LOGICAL_EXPRESSION_FOCUS),
        (&[Kind::UnsupportedIterableToArray], 200),
        BABEL_7_29_IE11_ROLLUP_TERSER_FOR_OF,
    ),
    variant(
        Kind::Regenerator,
        0x967c_1291_eeda_e607_470a_84d7_7bf0_7d57,
        (0, 24),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[Kind::RegeneratorDefine], 796),
        BABEL_7_28_IE11_GENERATORS,
    ),
    variant(
        Kind::Regenerator,
        0x448e_4e0f_264b_279a_aeb3_737a_ce3a_18a4,
        (0, 24),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[Kind::RegeneratorDefine], 789),
        BABEL_7_29_IE11_GENERATORS,
    ),
    variant(
        Kind::Regenerator,
        0x74ec_85ed_dd1c_da45_2d05_90f5_e365_702b,
        (0, 24),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[Kind::RegeneratorDefine], 794),
        BABEL_7_28_IE11_TERSER_GENERATORS,
    ),
    variant(
        Kind::Regenerator,
        0xb6e9_2040_8e33_1e47_2b51_9b17_1b65_a530,
        (0, 24),
        (VARIABLE_DECLARATION, NO_FOCUS),
        (&[Kind::RegeneratorDefine], 787),
        BABEL_7_29_IE11_TERSER_GENERATORS,
    ),
    variant(
        Kind::RegeneratorDefine,
        0xc11d_f564_ff3e_2b04_c8e0_adf6_1182_ca5a,
        (4, 4),
        (VARIABLE_DECLARATION, STATIC_MEMBER_FOCUS),
        (&[], 132),
        BABEL_7_29_IE11_GENERATORS,
    ),
    variant(
        Kind::RegeneratorDefine,
        0xc4d5_b2fe_7af5_7057_a90a_ca63_e0e2_5bdb,
        (4, 4),
        (VARIABLE_DECLARATION, STATIC_MEMBER_FOCUS),
        (&[], 131),
        BABEL_7_29_IE11_TERSER_GENERATORS,
    ),
    variant(
        Kind::AsyncToGenerator,
        0x31fe_a126_9765_5ce2_a156_181e_a7c0_5b9e,
        (1, 1),
        (RETURN_STATEMENT, ARROW_OR_FUNCTION_FOCUS),
        (&[Kind::AsyncGeneratorStep], 73),
        BABEL_7_29_IE11_ASYNC_AWAIT,
    ),
    variant(
        Kind::AsyncGeneratorStep,
        0x27e9_f47b_e830_ea4d_e8bd_3cd9_749c_18ff,
        (7, 2),
        (TRY_STATEMENT, NO_FOCUS),
        (&[], 64),
        BABEL_7_29_IE11_ASYNC_AWAIT,
    ),
];
