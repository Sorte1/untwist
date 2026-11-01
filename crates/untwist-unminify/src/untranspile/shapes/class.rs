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
    hash: u64,
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
        0x08fd_1989_60ea_75c3,
        (2, 1),
        (IF, UNARY),
        &[],
        BABEL,
    ),
    variant(
        Kind::CreateClass,
        0xbbe5_9c29_b5b7_fd46,
        (3, 4),
        (IF, IDENTIFIER),
        &[Kind::DefineProperties],
        BABEL,
    ),
    variant(
        Kind::CreateClassOpen,
        0x5b47_0e79_d078_c972,
        (3, 3),
        (IF, IDENTIFIER),
        &[Kind::DefinePropertiesPlain],
        SWC,
    ),
    variant(
        Kind::DefineProperties,
        0x8336_9a68_2323_b69e,
        (2, 1),
        (FOR, DECLARED_FOCUS),
        &[Kind::ToPropertyKey],
        BABEL,
    ),
    variant(
        Kind::DefinePropertiesPlain,
        0x8050_543b_6100_2c9f,
        (2, 1),
        (FOR, DECLARED_FOCUS),
        &[],
        SWC,
    ),
    variant(
        Kind::Inherits,
        0x4d14_faab_f22b_6a7d,
        (2, 4),
        (IF, LOGICAL),
        &[Kind::SetPrototypeOf],
        BABEL,
    ),
    variant(
        Kind::InheritsOpen,
        0x7a4e_4edd_7ed0_0b1f,
        (2, 3),
        (IF, LOGICAL),
        &[Kind::SetPrototypeOf],
        SWC,
    ),
    variant(
        Kind::InheritsOpen,
        0x2f64_b5ce_91d7_9521,
        (2, 3),
        (IF, LOGICAL),
        &[Kind::SetPrototypeOf],
        SWC_MIN,
    ),
    variant(
        Kind::SetPrototypeOf,
        0xf541_f68e_4940_da7b,
        (2, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        BABEL,
    ),
    variant(
        Kind::SetPrototypeOf,
        0x82fd_dbb6_c6c3_6c1a,
        (2, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC,
    ),
    variant(
        Kind::SetPrototypeOf,
        0x6800_42c0_ed77_6c03,
        (2, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC_MIN,
    ),
    variant(
        Kind::GetPrototypeOf,
        0xea94_fff3_fff6_cf7f,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        BABEL,
    ),
    variant(
        Kind::GetPrototypeOf,
        0x0a9d_eb58_1b4c_9ae4,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC,
    ),
    variant(
        Kind::GetPrototypeOf,
        0x7226_23aa_3c42_2b36,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        SWC_MIN,
    ),
    variant(
        Kind::CallSuper,
        0x44f1_4d67_01c2_8c8d,
        (3, 2),
        (EXPRESSION, ASSIGNMENT),
        CALL_SUPER_PARTS,
        BABEL,
    ),
    variant(
        Kind::CallSuper,
        0xa64d_45fd_751b_ee23,
        (3, 2),
        (EXPRESSION, ASSIGNMENT),
        CALL_SUPER_INLINED_PARTS,
        TERSER,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x3d53_6a9f_e443_1e0c,
        (2, 3),
        (IF, LOGICAL),
        RETURN_PARTS,
        BABEL,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x81d7_2ca8_4889_2178,
        (2, 2),
        (IF, LOGICAL),
        RETURN_PARTS,
        SWC,
    ),
    variant(
        Kind::PossibleConstructorReturn,
        0x1c3a_32fc_42c4_b65a,
        (2, 1),
        (IF, LOGICAL),
        RETURN_PARTS,
        SWC_MIN,
    ),
    variant(
        Kind::AssertThisInitialized,
        0x4e0e_43e4_c824_dda6,
        (1, 2),
        (IF, BINARY),
        &[],
        BABEL,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0x0402_c61b_edbe_a002,
        (0, 2),
        (TRY, NO_FOCUS),
        &[],
        BABEL,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0x3798_2ef0_a76f_6c7c,
        (0, 2),
        (TRY, NO_FOCUS),
        &[],
        BABEL_MIN,
    ),
    variant(
        Kind::IsNativeReflectConstruct,
        0xacb6_9721_9c95_e40c,
        (0, 2),
        (TRY, NO_FOCUS),
        &[],
        SWC,
    ),
    variant(
        Kind::ToPropertyKey,
        0xafb9_b15d_4c6a_f381,
        (1, 2),
        (DECLARATION, CALL),
        &[Kind::Typeof],
        TERSER_FIELDS,
    ),
    variant(
        Kind::Typeof,
        0x010e_3a19_4f46_efa6,
        (1, 1),
        (IF, LOGICAL),
        &[],
        SWC,
    ),
    variant(
        Kind::Typeof,
        0xbb46_c8f9_bae4_cc68,
        (1, 2),
        (EXPRESSION, ASSIGNMENT),
        &[],
        TERSER_FIELDS,
    ),
    variant(
        Kind::PublicField,
        0x2922_a0e5_57ee_c94e,
        (3, 1),
        (EXPRESSION_BODY, CALL),
        PUBLIC_FIELD_PARTS,
        ESBUILD_FIELDS,
    ),
    variant(
        Kind::PublicField,
        0xbbe0_89a9_998d_8d44,
        (3, 1),
        (EXPRESSION_BODY, CALL),
        PUBLIC_FIELD_PARTS,
        ESBUILD_FIELDS_MIN,
    ),
    variant(
        Kind::PublicField,
        0x6596_329c_f34f_2572,
        (3, 1),
        (RETURN, CALL),
        PUBLIC_FIELD_INLINED_PARTS,
        TERSER_FIELDS,
    ),
    variant(
        Kind::Extends,
        0xfa61_21dd_ff9d_9cc9,
        (0, 2),
        (DECLARATION, FUNCTION),
        &[],
        TS,
    ),
    variant(
        Kind::Extends,
        0x77fa_e3fc_c650_9cbf,
        (0, 2),
        (DECLARATION, FUNCTION),
        &[],
        TS_MIN,
    ),
];
