use oxc_ast::ast::{Expression, Statement};

pub const HOOK_COUNT: usize = 16;

pub const HOOK_NAMES: [&str; HOOK_COUNT] = [
    "ENTER_PROGRAM",
    "EXIT_PROGRAM",
    "ENTER_STATEMENTS",
    "EXIT_STATEMENTS",
    "ENTER_STATEMENT",
    "EXIT_STATEMENT",
    "ENTER_EXPRESSION",
    "EXIT_EXPRESSION",
    "EXIT_FUNCTION",
    "EXIT_ARROW_FUNCTION_EXPRESSION",
    "EXIT_VARIABLE_DECLARATION",
    "EXIT_OBJECT_PROPERTY",
    "EXIT_METHOD_DEFINITION",
    "EXIT_PROPERTY_DEFINITION",
    "EXIT_TEMPLATE_LITERAL",
    "EXIT_ASSIGNMENT_TARGET",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interest {
    hooks: u16,
    expressions: ExpressionVariants,
    statements: StatementVariants,
}

impl Interest {
    pub const NONE: Interest = Interest::hooks(0);
    pub const ENTER_PROGRAM: Interest = Interest::hooks(1 << 0);
    pub const EXIT_PROGRAM: Interest = Interest::hooks(1 << 1);
    pub const ENTER_STATEMENTS: Interest = Interest::hooks(1 << 2);
    pub const EXIT_STATEMENTS: Interest = Interest::hooks(1 << 3);
    pub const ENTER_STATEMENT: Interest = Interest::hooks(1 << 4);
    pub const EXIT_STATEMENT: Interest = Interest::hooks(1 << 5);
    pub const ENTER_EXPRESSION: Interest = Interest::hooks(1 << 6);
    pub const EXIT_EXPRESSION: Interest = Interest::hooks(1 << 7);
    pub const EXIT_FUNCTION: Interest = Interest::hooks(1 << 8);
    pub const EXIT_ARROW_FUNCTION_EXPRESSION: Interest = Interest::hooks(1 << 9);
    pub const EXIT_VARIABLE_DECLARATION: Interest = Interest::hooks(1 << 10);
    pub const EXIT_OBJECT_PROPERTY: Interest = Interest::hooks(1 << 11);
    pub const EXIT_METHOD_DEFINITION: Interest = Interest::hooks(1 << 12);
    pub const EXIT_PROPERTY_DEFINITION: Interest = Interest::hooks(1 << 13);
    pub const EXIT_TEMPLATE_LITERAL: Interest = Interest::hooks(1 << 14);
    pub const EXIT_ASSIGNMENT_TARGET: Interest = Interest::hooks(1 << 15);
    pub const ALL: Interest = Interest::hooks(u16::MAX);

    const fn hooks(hooks: u16) -> Interest {
        Interest {
            hooks,
            expressions: ExpressionVariants::ALL,
            statements: StatementVariants::ALL,
        }
    }

    pub const fn union(self, other: Interest) -> Interest {
        Interest {
            hooks: self.hooks | other.hooks,
            expressions: self.expressions.union(other.expressions),
            statements: self.statements.union(other.statements),
        }
    }

    pub const fn on_expressions(self, expressions: ExpressionVariants) -> Interest {
        Interest {
            expressions,
            ..self
        }
    }

    pub const fn on_statements(self, statements: StatementVariants) -> Interest {
        Interest { statements, ..self }
    }

    pub const fn expressions(self) -> ExpressionVariants {
        self.expressions
    }

    pub const fn statements(self) -> StatementVariants {
        self.statements
    }

    pub const fn includes(self, other: Interest) -> bool {
        self.hooks & other.hooks == other.hooks
    }

    pub const fn slot(self) -> usize {
        self.hooks.trailing_zeros() as usize
    }

    pub(crate) const fn hook_bits(self) -> u16 {
        self.hooks
    }

    pub(crate) const fn bit_at(slot: usize) -> Interest {
        Interest::hooks(1 << slot)
    }

    pub fn named(self) -> Vec<&'static str> {
        HOOK_NAMES
            .iter()
            .enumerate()
            .filter(|(slot, _)| self.hooks & (1 << slot) != 0)
            .map(|(_, name)| *name)
            .collect()
    }
}

impl std::ops::BitOr for Interest {
    type Output = Interest;

    fn bitor(self, rhs: Interest) -> Interest {
        self.union(rhs)
    }
}

macro_rules! variant_kinds {
    (
        $kind:ident, $set:ident, $bits:ty, $node:ident, $count:ident = $len:literal,
        [$($variant:ident = $value:literal),* $(,)?]
    ) => {
        pub const $count: usize = $len;

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $kind {
            $($variant = $value,)*
        }

        impl $kind {
            #[inline]
            pub fn of(node: &$node<'_>) -> $kind {
                match node {
                    $($node::$variant(_) => $kind::$variant,)*
                }
            }

            #[inline]
            pub const fn index(self) -> usize {
                self as usize
            }

            pub const fn from_index(index: usize) -> Option<$kind> {
                match index {
                    $($value => Some($kind::$variant),)*
                    _ => None,
                }
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $set($bits);

        impl $set {
            pub const ALL: $set = $set(<$bits>::MAX);
            pub const NONE: $set = $set(0);

            pub const fn of(kinds: &[$kind]) -> $set {
                let one: $bits = 1;
                let mut bits: $bits = 0;
                let mut at = 0;
                while at < kinds.len() {
                    bits |= one << (kinds[at] as u32);
                    at += 1;
                }
                $set(bits)
            }

            pub const fn union(self, other: $set) -> $set {
                $set(self.0 | other.0)
            }

            pub const fn includes(self, kind: $kind) -> bool {
                self.includes_index(kind as usize)
            }

            pub const fn includes_index(self, index: usize) -> bool {
                index < <$bits>::BITS as usize && (self.0 >> index) & 1 == 1
            }
        }
    };
}

variant_kinds!(
    ExpressionKind,
    ExpressionVariants,
    u64,
    Expression,
    EXPRESSION_KINDS = 51,
    [
        BooleanLiteral = 0,
        NullLiteral = 1,
        NumericLiteral = 2,
        BigIntLiteral = 3,
        RegExpLiteral = 4,
        StringLiteral = 5,
        TemplateLiteral = 6,
        Identifier = 7,
        Super = 8,
        ArrayExpression = 9,
        ArrowFunctionExpression = 10,
        AssignmentExpression = 11,
        AwaitExpression = 12,
        BinaryExpression = 13,
        CallExpression = 14,
        ChainExpression = 15,
        ClassExpression = 16,
        ConditionalExpression = 17,
        FunctionExpression = 18,
        ImportExpression = 19,
        LogicalExpression = 20,
        NewExpression = 21,
        ObjectExpression = 22,
        ParenthesizedExpression = 23,
        SequenceExpression = 24,
        TaggedTemplateExpression = 25,
        ThisExpression = 26,
        UnaryExpression = 27,
        UpdateExpression = 28,
        YieldExpression = 29,
        PrivateInExpression = 30,
        ImportMeta = 31,
        NewTarget = 32,
        JSXElement = 33,
        JSXFragment = 34,
        TSAsExpression = 35,
        TSSatisfiesExpression = 36,
        TSTypeAssertion = 37,
        TSNonNullExpression = 38,
        TSInstantiationExpression = 39,
        V8IntrinsicExpression = 40,
        ComputedMemberExpression = 48,
        StaticMemberExpression = 49,
        PrivateFieldExpression = 50,
    ]
);

variant_kinds!(
    StatementKind,
    StatementVariants,
    u128,
    Statement,
    STATEMENT_KINDS = 72,
    [
        BlockStatement = 0,
        BreakStatement = 1,
        ContinueStatement = 2,
        DebuggerStatement = 3,
        DoWhileStatement = 4,
        EmptyStatement = 5,
        ExpressionStatement = 6,
        ForInStatement = 7,
        ForOfStatement = 8,
        ForStatement = 9,
        IfStatement = 10,
        LabeledStatement = 11,
        ReturnStatement = 12,
        SwitchStatement = 13,
        ThrowStatement = 14,
        TryStatement = 15,
        WhileStatement = 16,
        WithStatement = 17,
        VariableDeclaration = 32,
        FunctionDeclaration = 33,
        ClassDeclaration = 34,
        TSTypeAliasDeclaration = 35,
        TSInterfaceDeclaration = 36,
        TSEnumDeclaration = 37,
        TSExternalModuleDeclaration = 38,
        TSNamespaceDeclaration = 39,
        TSGlobalDeclaration = 40,
        TSImportEqualsDeclaration = 41,
        ImportDeclaration = 64,
        ExportAllDeclaration = 65,
        ExportDefaultDeclaration = 66,
        ExportDeclaration = 67,
        ExportNamedDeclaration = 68,
        ExportFromDeclaration = 69,
        TSExportAssignment = 70,
        TSNamespaceExportDeclaration = 71,
    ]
);
