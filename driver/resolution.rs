use std::collections::HashMap;
use super::*;
use language::*;
use common::*;

#[derive(Clone)]
struct VariableInfo {
    unique_name: String,
    from_current_block: bool,
}

type VariableMap = HashMap<String, VariableInfo>;

pub struct VariableResolutionCheck;

impl SemanticCheck for VariableResolutionCheck {
    fn analyze(ast: &mut AstProgram, ctx: &mut SemanticContext) {
        let mut variable_map = HashMap::new();
        let body = ast.function_definition_mut().body_mut();
        *body = resolve_block(body.clone(), ctx, &mut variable_map);
    }
}

fn resolve_block(
    block: AstBlock,
    ctx: &mut SemanticContext,
    parent_map: &mut VariableMap,
) -> AstBlock {
    let mut local_map = copy_variable_map(parent_map);
    let mut new_block = block.clone();

    for (i, item) in block.block_items().iter().enumerate() {
        let new_item = match item {
            AstBlockItem::Declaration(decl) => {
                AstBlockItem::Declaration(resolve_declaration(decl.clone(), ctx, &mut local_map))
            }
            AstBlockItem::Statement(stmt) => {
                AstBlockItem::Statement(resolve_statement(stmt.clone(), ctx, &mut local_map))
            }
        };
        new_block.block_items_mut()[i] = new_item;
    }

    new_block
}

fn resolve_declaration(
    decl: AstDeclaration,
    ctx: &mut SemanticContext,
    variable_map: &mut VariableMap,
) -> AstDeclaration {
    let name = decl.name().clone();

    if let Some(existing) = variable_map.get(&name) {
        if existing.from_current_block {
            ctx.diagnostics_mut().push(Diagnostic::error(
                decl.get_span(),
                DiagnosticKind::DuplicateVariableDeclaration { name: name.clone() },
            ));
        }
    }

    let unique_name = ctx.temp_gen_mut().temp_from(name.clone());
    variable_map.insert(
        name,
        VariableInfo {
            unique_name: unique_name.clone(),
            from_current_block: true,
        },
    );

    let new_init = decl
        .init()
        .as_ref()
        .map(|e| resolve_expression(e.clone(), ctx, variable_map));

    AstDeclaration::new(unique_name, new_init, decl.get_span())
}

fn resolve_statement(
    stmt: AstStatement,
    ctx: &mut SemanticContext,
    variable_map: &mut VariableMap,
) -> AstStatement {
    match stmt {
        AstStatement::Return { expression, span } => AstStatement::Return {
            expression: resolve_expression(expression, ctx, variable_map),
            span
        },
        AstStatement::Expression { expression, span } => AstStatement::Expression {
            expression: resolve_expression(expression, ctx, variable_map),
            span
        },
        AstStatement::If {
            condition,
            then_branch,
            else_branch,
            span
        } => AstStatement::If {
            condition: resolve_expression(condition, ctx, variable_map),
            then_branch: Box::new(resolve_statement(*then_branch, ctx, variable_map)),
            else_branch: else_branch.map(|branch| Box::new(resolve_statement(*branch, ctx, variable_map))),
            span
        },
        AstStatement::Compound { block, span } => {
            let mut new_map = copy_variable_map(variable_map);
            AstStatement::Compound {
                block: resolve_block(block, ctx, &mut new_map),
                span
            }
        }
        AstStatement::Break { label, span } => {
            if ctx.loop_depth == 0 {
                ctx.diagnostics.push(Diagnostic::error(
                    span,
                    DiagnosticKind::new_custom("break used outside of the loop".to_string()),
                ));
            }
            AstStatement::Break { label, span }
        }

        AstStatement::Continue { label, span }  => {
            if ctx.loop_depth == 0 {
                ctx.diagnostics.push(Diagnostic::error(
                    span,
                    DiagnosticKind::new_custom("continue used outside of the loop".to_string()),
                ));
            }
            AstStatement::Continue { label, span }
        }

        AstStatement::While { condition, body, label, span } => {
            let new_condition = resolve_expression(condition, ctx, variable_map);
            ctx.loop_depth += 1;
            let new_body = Box::new(resolve_statement(*body, ctx, variable_map));
            ctx.loop_depth -= 1;
            AstStatement::While {
                condition: new_condition,
                body: new_body,
                label,
                span
            }
        }
        AstStatement::DoWhile { condition, body, label, span } => {
            ctx.loop_depth += 1;
            let new_body = Box::new(resolve_statement(*body, ctx, variable_map));
            ctx.loop_depth -= 1;
            let new_condition = resolve_expression(condition, ctx, variable_map);
            AstStatement::DoWhile {
                condition: new_condition,
                body: new_body,
                label,
                span,
            }
        }
        AstStatement::For {
            for_init,
            condition,
            post,
            body,
            label,
            span,
        } => {
            let mut new_map = copy_variable_map(variable_map);
            let new_init = resolve_for_init(for_init, ctx, &mut new_map);
            let new_condition = resolve_optional_expression(condition, ctx, &new_map);
            let new_post = resolve_optional_expression(post, ctx, &new_map);

            ctx.loop_depth += 1;
            let new_body = Box::new(resolve_statement(*body, ctx, &mut new_map));
            ctx.loop_depth -= 1;

            AstStatement::For {
                for_init: new_init,
                condition: new_condition,
                post: new_post,
                body: new_body,
                label,
                span,
            }
        }
        AstStatement::Null => AstStatement::Null,
    }
}

fn resolve_optional_expression(
    expr: Option<AstExpression>,
    ctx: &mut SemanticContext,
    variable_map: &VariableMap,
) -> Option<AstExpression> {
    expr.map(|e| resolve_expression(e, ctx, variable_map))
}

fn resolve_for_init(
    init: AstForInit,
    ctx: &mut SemanticContext,
    variable_map: &mut VariableMap,
) -> AstForInit {
    match init {
        AstForInit::InitExpression(expr_opt) => {
            AstForInit::InitExpression(resolve_optional_expression(expr_opt, ctx, variable_map))
        }
        AstForInit::InitDeclaration(decl) => {
            AstForInit::InitDeclaration(resolve_declaration(decl, ctx, variable_map))
        }
    }
}

fn resolve_expression(
    expr: AstExpression,
    ctx: &mut SemanticContext,
    variable_map: &VariableMap,
) -> AstExpression {
    match expr {
        AstExpression::Var { identifier, span } => {
            if let Some(info) = variable_map.get(&identifier) {
                AstExpression::Var {
                    identifier: info.unique_name.clone(),
                    span,
                }
            } else {
                ctx.diagnostics_mut().push(Diagnostic::error(
                    span,
                    DiagnosticKind::UseOfUndeclaredVariable { name: identifier.clone() },
                ));
                AstExpression::Var { identifier, span }
            }
        }
        AstExpression::Assignment { left, right, span } => {
            let resolved_left = resolve_expression(*left, ctx, variable_map);
            let resolved_right = resolve_expression(*right, ctx, variable_map);
            if let AstExpression::Var { .. } = resolved_left {
                AstExpression::Assignment {
                    left: Box::new(resolved_left),
                    right: Box::new(resolved_right),
                    span,
                }
            } else {
                ctx.diagnostics_mut().push(Diagnostic::error(
                    resolved_left.get_span(),
                    DiagnosticKind::Custom("invalid lvalue in assignment".to_string()),
                ));
                AstExpression::Assignment {
                    left: Box::new(resolved_left),
                    right: Box::new(resolved_right),
                    span,
                }
            }
        }
        AstExpression::Unary {
            operator,
            operand,
            span,
        } => AstExpression::Unary {
            operator,
            operand: Box::new(resolve_expression(*operand, ctx, variable_map)),
            span,
        },
        AstExpression::Binary {
            operator,
            left,
            right,
            span,
        } => AstExpression::Binary {
            operator,
            left: Box::new(resolve_expression(*left, ctx, variable_map)),
            right: Box::new(resolve_expression(*right, ctx, variable_map)),
            span,
        },
        AstExpression::Conditional {
            condition,
            then_branch,
            else_branch,
            span,
        } => AstExpression::Conditional {
            condition: Box::new(resolve_expression(*condition, ctx, variable_map)),
            then_branch: Box::new(resolve_expression(*then_branch, ctx, variable_map)),
            else_branch: Box::new(resolve_expression(*else_branch, ctx, variable_map)),
            span,
        },
        AstExpression::Constant { .. } => expr,
    }
}

fn copy_variable_map(map: &VariableMap) -> VariableMap {
    map.iter()
        .map(|(name, info)| {
            (
                name.clone(),
                VariableInfo {
                    unique_name: info.unique_name.clone(),
                    from_current_block: false,
                },
            )
        })
        .collect()
}
