//! Node kind strings from the tree-sitter-php grammar.
//!
//! Centralised so that a grammar upgrade is a one-file change rather than a
//! scavenger hunt through the walker. `tests/grammar.rs` parses a fixture
//! exercising every construct and fails if any kind here no longer appears,
//! which is what catches a silently renamed node after a `cargo update`.

pub const IF_STATEMENT: &str = "if_statement";
pub const ELSE_IF_CLAUSE: &str = "else_if_clause";
pub const ELSE_CLAUSE: &str = "else_clause";
pub const CONDITIONAL_EXPRESSION: &str = "conditional_expression";

pub const SWITCH_STATEMENT: &str = "switch_statement";
pub const MATCH_EXPRESSION: &str = "match_expression";

pub const FOR_STATEMENT: &str = "for_statement";
pub const FOREACH_STATEMENT: &str = "foreach_statement";
pub const WHILE_STATEMENT: &str = "while_statement";
pub const DO_STATEMENT: &str = "do_statement";

pub const CATCH_CLAUSE: &str = "catch_clause";

pub const GOTO_STATEMENT: &str = "goto_statement";
pub const BREAK_STATEMENT: &str = "break_statement";
pub const CONTINUE_STATEMENT: &str = "continue_statement";

pub const BINARY_EXPRESSION: &str = "binary_expression";
pub const INTEGER: &str = "integer";

pub const FUNCTION_DEFINITION: &str = "function_definition";
pub const METHOD_DECLARATION: &str = "method_declaration";
pub const ARROW_FUNCTION: &str = "arrow_function";

/// The grammar renamed anonymous functions between releases, so both spellings
/// are accepted rather than pinning users to one grammar version.
pub const ANONYMOUS_FUNCTION: &str = "anonymous_function";
pub const ANONYMOUS_FUNCTION_CREATION: &str = "anonymous_function_creation_expression";

pub const FUNCTION_CALL: &str = "function_call_expression";
pub const MEMBER_CALL: &str = "member_call_expression";
pub const SCOPED_CALL: &str = "scoped_call_expression";

pub const COMMENT: &str = "comment";
pub const ATTRIBUTE_LIST: &str = "attribute_list";

pub const CLASS_DECLARATION: &str = "class_declaration";
pub const INTERFACE_DECLARATION: &str = "interface_declaration";
pub const TRAIT_DECLARATION: &str = "trait_declaration";
pub const ENUM_DECLARATION: &str = "enum_declaration";

/// Closures nested inside a unit roll their score up into it, per the spec,
/// so they are not units in their own right.
#[must_use]
pub fn is_scoring_unit(kind: &str) -> bool {
    matches!(kind, FUNCTION_DEFINITION | METHOD_DECLARATION)
}

/// Two classes in one file can each declare `process()`, so the enclosing type
/// is what makes a finding addressable.
#[must_use]
pub fn is_type_declaration(kind: &str) -> bool {
    matches!(
        kind,
        CLASS_DECLARATION | INTERFACE_DECLARATION | TRAIT_DECLARATION | ENUM_DECLARATION
    )
}

/// Function-likes that raise the nesting level without scoring in their own right.
#[must_use]
pub fn is_nested_function(kind: &str) -> bool {
    matches!(
        kind,
        ANONYMOUS_FUNCTION | ANONYMOUS_FUNCTION_CREATION | ARROW_FUNCTION | FUNCTION_DEFINITION
    )
}
