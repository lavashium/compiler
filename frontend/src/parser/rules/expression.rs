use super::*;
use language::*;

macro_rules! precedence {
    ($operator:expr; $($op:ident => $val:expr),* $(,)?) => {
        match $operator {
            $(
                OperatorKind::$op => $val,
            )*
            _ => 0
        }
    }
}

fn get_precedence(operator: &OperatorKind) -> u8 {
    precedence!(operator;
        Asterisk      => 50,
        Slash         => 50,
        Percent       => 50,
        Plus          => 45,
        Minus         => 45,
        LessThan      => 35,
        LessEqual     => 35,
        GreaterThan   => 35,
        GreaterEqual  => 35,
        EqualEqual    => 30,
        NotEqual      => 30,
        LogicalAnd    => 10,
        LogicalOr     => 5,
    )
}

pub trait ExpressionParser {
    fn parse_expression(&mut self) -> ParseResult<AstExpression>;
    fn parse_binary_expression(&mut self, min_prec: u8) -> ParseResult<AstExpression>;
}

impl<'a> ExpressionParser for ParserRules<'a> {
    fn parse_expression(&mut self) -> ParseResult<AstExpression> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, min_prec: u8) -> ParseResult<AstExpression> {
        let mut lhs = {
            match self.parser.source_tokens.peek()?.kind() {
                TokenKind::Constant(_) => {
                    let constant = self.unwrap_constant()?;
                    AstExpression::Constant { constant }
                }

                TokenKind::Operator(op @ OperatorKind::Tilde) |
                TokenKind::Operator(op @ OperatorKind::Exclamation) |
                TokenKind::Operator(op @ OperatorKind::Minus) => {
                    let operator = op.to_unary()?.clone();

                    self.parser.source_tokens.consume();

                    let operand = self.parse_binary_expression(100)?;
                    AstExpression::Unary {
                        operator,
                        operand: Box::new(operand),
                    }
                }

                TokenKind::Punctuation(PunctuationKind::OpenParen) => {
                    self.expect(token_punctuation!(OpenParen))?;
                    let inner = self.parse_expression()?;
                    self.expect(token_punctuation!(CloseParen))?;
                    inner
                }

                _ => {
                    let token = self.parser.source_tokens.peek()?;
                    self.diagnostics.push(
                        Diagnostic::error(
                            token.get_span(),
                            DiagnosticKind::UnknownToken(token.clone()),
                        )
                        .with(Diagnostic::note(
                            token.get_span(),
                            "expected an expression here",
                        )),
                    );
                    return None;
                }
            }
        };

        loop {
            let next_token = match self.parser.source_tokens.peek() {
                Some(tok) => tok,
                None => break,
            };

            let op_kind = match next_token.kind() {
                TokenKind::Operator(op) => op.clone(),
                _ => break,
            };

            let prec = get_precedence(&op_kind);
            if prec < min_prec {
                break;
            }

            self.parser.source_tokens.consume()?;

            let rhs = self.parse_binary_expression(prec+1)?;

            let op_kind = op_kind.to_binary()?;

            lhs = AstExpression::Binary {
                operator: op_kind,
                left: Box::new(lhs),
                right: Box::new(rhs),
            };
        }

        Some(lhs)
    }
}