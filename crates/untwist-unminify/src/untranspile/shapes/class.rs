use untwist_core::pass::{ExpressionKind, StatementKind};

use super::{Alias, Kind, Variant};

const BABEL: &str = "fixtures/untranspile/babel/classes.7.29.js";
const BABEL_MIN: &str = "fixtures/untranspile/babel/classes.7.29.min.js";
const SWC: &str = "fixtures/untranspile/swc/classes.es5.js";
const SWC_MIN: &str = "fixtures/untranspile/swc/classes.es5.min.js";
const TERSER: &str = "fixtures/untranspile/terser/classes.safari9.js";
const TERSER_FIELDS: &str = "fixtures/untranspile/terser/class-fields.esbuild.ie11.js";
const ESBUILD_FIELDS: &str = "fixtures/untranspile/esbuild/class-fields.0.28.1-es2015.js";
const ESBUILD_FIELDS_MIN: &str = "fixtures/untranspile/esbuild/class-fields.0.28.1-es2015.min.js";
const TS: &str = "fixtures/untranspile/ts/classes.es5.js";
const TS_MIN: &str = "fixtures/untranspile/ts/classes.es5.min.js";

const DECLARATION: u8 = StatementKind::VariableDeclaration as u8;
const EXPRESSION: u8 = StatementKind::ExpressionStatement as u8;
const FOR: u8 = StatementKind::ForStatement as u8;
const IF: u8 = StatementKind::IfStatement as u8;
const RETURN: u8 = StatementKind::ReturnStatement as u8;
const TRY: u8 = StatementKind::TryStatement as u8;
const EXPRESSION_BODY: u8 = u8::MAX;
const DECLARED_FOCUS: u8 = u8::MAX - 1;

const ASSIGNMENT: u8 = ExpressionKind::AssignmentExpression as u8;
const BINARY: u8 = ExpressionKind::BinaryExpression as u8;
const CALL: u8 = ExpressionKind::CallExpression as u8;
const FUNCTION: u8 = ExpressionKind::FunctionExpression as u8;
const IDENTIFIER: u8 = ExpressionKind::Identifier as u8;
const LOGICAL: u8 = ExpressionKind::LogicalExpression as u8;
const UNARY: u8 = ExpressionKind::UnaryExpression as u8;
const NO_FOCUS: u8 = u8::MAX;

const CALL_SUPER_PARTS: &[Kind] = &[
    Kind::GetPrototypeOf,
    Kind::PossibleConstructorReturn,
    Kind::IsNativeReflectConstruct,
];
const CALL_SUPER_INLINED_PARTS: &[Kind] = &[Kind::GetPrototypeOf, Kind::IsNativeReflectConstruct];
const RETURN_PARTS: &[Kind] = &[Kind::Typeof, Kind::AssertThisInitialized];
const PUBLIC_FIELD_PARTS: &[Kind] = &[Kind::DefNormalProp];
const PUBLIC_FIELD_INLINED_PARTS: &[Kind] = &[Kind::Alias(Alias::DefineProperty), Kind::Typeof];

const fn variant(
    kind: Kind,
    hash: u128,
    (params, statements): (u8, u8),
    (lead, focus): (u8, u8),
    external: &'static [Kind],
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
        cap: super::TOKEN_CAP,
    }
}

pub const VARIANTS: &[Variant] = &[
    variant(
        Kind::ClassCallCheck,
        0x1ada_0562_2ff1_54be_8619_3f7d_9b6e_fd40,
        (2, 1),
        (IF, UNARY),
        &[],
        BABEL,
    ),
    variant(
        Kind::CreateClass,
        0x1c39_d850_8f1d_439a_8868_fbc1_2310_5898,
        (3, 4),
        (IF, IDENTIFIER),
        &[Kind::DefineProperties],
        BABEL,
    ),
    variant(
        Kind::CreateClassOpen,
        0x9650_b2b2_3d00_322c_364a_004a_dd56_9fa5,
        (3, 3),
        (IF, IDENTIFIER),
        &[Kind::DefinePropertiesPlain],
        SWC,
    ),
    variant(
        Kind::DefineProperties,
        0xb941_4cd2_f6cd_76a0_64d9_bbca_10f6_6ffd,
        (2, 1),
        (FOR, DECLARED_FOCUS),
        &[Kind::ToPropertyKey],
        BABEL,
    ),
    variant(
        Kind::DefinePropertiesPlain,
        0x5bfd_ff95_1b39_81ce_0841_e1a5_812a_d12a,
        (2, 1),
        (FOR, DECLARED_FOCUS),
        &[],
        SWC,
    ),
    variant(
        Kind::Inherits,
        0x7ff6_8b24_4ee0_4868_8648_ec85_2fb0_1622,
        (2, 4),
        (IF, LOGICAL),
        &[Kind::SetPrototypeOf],
        BABEL,
    ),
    variant(
        Kind::InheritsOpen,
        0xc714_0489_05ad_1010_eb72_09ab_613f_5291,
        (2, 3),
        (IF, LOGICAL),
        &[Kind::SetPrototypeOf],
        SWC,
    ),
    variant(
        Kind::InheritsOpen,
        0xd90b_4b44_ac66_81a0_500c_9238_ef37_7d43,
        (2, 3),
        (IF, LOGICAL),
        &[Kind::SetPrototypeOf],
        SWC_MIN,
    ),
    variant(
        Kind::SetPrototypeOf,
        0xc413_03d7_c9bc_0b0c_0736_43fa_78da_5bb1,
        (2, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        BABEL,
    ),
    variant(
        Kind::SetPrototypeOf,
        0xeeba_8b3e_840a_578a_20d5_c615_8cda_daca,
        (2, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC,
    ),
    variant(
        Kind::SetPrototypeOf,
        0xd6d6_d132_62f7_3084_c4c9_2da8_93d3_84c0,
        (2, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC_MIN,
    ),
    variant(
        Kind::GetPrototypeOf,
        0xe350_1d77_fcb1_07fa_90eb_743b_09fb_1f5e,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        BABEL,
    ),
    variant(
        Kind::GetPrototypeOf,
        0x2e56_38fe_8ec2_4f12_f894_599f_57ad_c957,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC,
    ),
    variant(
        Kind::GetPrototypeOf,
        0x841d_ecb0_7a96_cccc_4f43_a31a_3c5f_7856,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC_MIN,
    ),
    variant(
        Kind::CallSuper,
        0xb553_fdff_764c_db4f_f2da_6b03_56f8_12b6,
        (3, 2),
        (EXPRESSION, ASSIGNMENT),
        CALL_SUPER_PARTS,
        BABEL,
    ),
    variant(
        Kind::CallSuper,
        0x9f2e_e014_3e99_ada2_7436_d677_2c21_c379,
        (3, 2),
        (EXPRESSION, ASSIGNMENT),
        CALL_SUPER_INLINED_PARTS,
        TERSER,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x7a2b_2502_80f6_8617_1e9a_210d_13d0_cfd1,
        (2, 3),
        (IF, LOGICAL),
        RETURN_PARTS,
        BABEL,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x414e_e8dd_96a3_096d_3803_ede9_543c_5110,
        (2, 2),
        (IF, LOGICAL),
        RETURN_PARTS,
        SWC,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x2fad_3de7_0ecf_3b1a_3210_0c48_a8e1_45e5,
        (2, 1),
        (IF, LOGICAL),
        RETURN_PARTS,
        SWC_MIN,
    ),
    variant(
        Kind::AssertThisInitialized,
        0x7bcd_1fd1_c09b_c3a5_7b87_5c5e_2b45_8e4c,
        (1, 2),
        (IF, BINARY),
        &[],
        BABEL,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0xb934_881f_7727_45e7_06b0_159d_9753_a6d3,
        (0, 2),
        (TRY, NO_FOCUS),
        &[],
        BABEL,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0x1250_ca54_5904_b527_a208_fe55_566f_85ab,
        (0, 2),
        (TRY, NO_FOCUS),
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0xf6b3_dc1a_9e3d_0573_ba68_4c25_8080_514a,
        (0, 2),
        (TRY, NO_FOCUS),
        &[],
        SWC,
    ),
    variant(
        Kind::ToPropertyKey,
        0xd026_5740_8a27_d444_b941_5074_9716_98ee,
        (1, 2),
        (DECLARATION, CALL),
        &[Kind::Typeof],
        TERSER_FIELDS,
    ),
    variant(
        Kind::Typeof,
        0x667e_33cb_db11_4a7a_c3fb_027b_4ae8_994e,
        (1, 1),
        (IF, LOGICAL),
        &[],
        SWC,
    ),
    variant(
        Kind::Typeof,
        0xc0ac_76a1_e746_7abd_75f5_c63d_8d5b_8e83,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        TERSER_FIELDS,
    ),
    variant(
        Kind::PublicField,
        0xce2a_294b_e631_21f9_9984_8828_6184_960c,
        (3, 1),
        (EXPRESSION_BODY, CALL),
        PUBLIC_FIELD_PARTS,
        ESBUILD_FIELDS,
    ),
    variant(
        Kind::PublicField,
        0xd6fe_4f44_1ee9_8d7f_0b34_231e_81a5_7c8f,
        (3, 1),
        (EXPRESSION_BODY, CALL),
        PUBLIC_FIELD_PARTS,
        ESBUILD_FIELDS_MIN,
    ),
    variant(
        Kind::PublicField,
        0x3664_3d65_40d7_4241_ad68_d83a_aa8c_20e5,
        (3, 1),
        (RETURN, CALL),
        PUBLIC_FIELD_INLINED_PARTS,
        TERSER_FIELDS,
    ),
    variant(
        Kind::Extends,
        0xdfc6_f084_b6d4_3b82_d2ab_060a_7538_3b68,
        (0, 2),
        (DECLARATION, FUNCTION),
        &[],
        TS,
    ),
    variant(
        Kind::Extends,
        0x0cf4_0b09_a386_81b9_fcf5_c4b9_9404_2aec,
        (0, 2),
        (DECLARATION, FUNCTION),
        &[],
        TS_MIN,
    ),
];
