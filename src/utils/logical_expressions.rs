use std::fmt::Debug;
use thiserror::Error;

#[cfg(test)]
#[path = "./logical_expressions.tests.rs"]
mod logical_expressions_tests;

/// Possible errors from expanding logical expression string slice.
#[derive(Error, Debug)]
pub enum ParserError {
    /// Operator was expected at the `index` position.
    #[error("expected operator at index {0}")]
    ExpectedOperator(usize),

    /// Operator was not expected at the `index` position.
    #[error("unexpected operator at index {0}")]
    UnexpectedOperator(usize),

    /// Closing bracket was expected for opening bracket at the `index` position.
    #[error("expected closing bracket for opening bracket at index {0}")]
    ExpectedClosingBracket(usize),

    /// Closing bracket was not expected at the `index` position.
    #[error("unexpected closing bracket at index {0}")]
    UnexpectedClosingBracket(usize),
}

/// Validates if the provided logical expression can be parsed.
pub fn validate(expression: &str) -> Result<(), ParserError> {
    let _ = tokenize(expression)?;
    Ok(())
}

/// Parses provided logical expression string slice.
pub fn parse(expression: &str) -> Result<Expression, ParserError> {
    let tokens = tokenize(expression)?;
    Ok(parse_tokens(tokens))
}

/// Possible operators for logical expression.
#[derive(Default, Debug, PartialEq)]
pub enum Operator {
    And,
    #[default]
    Or,
}

/// Parsed tokens united as an [`Expression`] that has `lhs` and `rhs` parts joined with the operator.\
/// **Note** that an expression can be also represented as the end, single value.
#[derive(Default, Debug)]
pub struct Expression {
    pub lhs: Option<Box<Expression>>,
    pub rhs: Option<Box<Expression>>,
    pub op: Operator,
    pub is_negation: bool,
    pub value: Option<String>,
}

impl Expression {
    /// Creates new [`Expression`] instance that represents an end value.
    pub fn new(s: &str, is_negation: bool) -> Self {
        Self {
            lhs: None,
            rhs: None,
            op: Operator::Or,
            is_negation,
            value: Some(s.trim().to_ascii_lowercase()),
        }
    }

    /// Returns `true` if [`Expression`] is complete: has both hand sides of the equation or is a value.
    pub fn is_complete(&self) -> bool {
        self.value.is_some() || (self.lhs.is_some() && self.rhs.is_some())
    }

    /// Returns `true` if [`Expression`] is in fact an end value.
    pub fn is_value(&self) -> bool {
        self.value.is_some()
    }

    /// Returns `true` if this [`Expression`] is a pointless wrapper (has only `lhs` side without negation).
    pub fn is_pointless(&self) -> bool {
        self.lhs.is_some() && self.rhs.is_none() && self.value.is_none() && !self.is_negation
    }

    /// Returns `true` if this [`Expression`] has only `lhs` side.
    pub fn has_only_lhs(&self) -> bool {
        self.lhs.is_some() && self.rhs.is_none() && self.value.is_none()
    }

    /// Pushes new [`Expression`] to the first empty hand side (left, then right).\
    /// **Note** that it returns `true` if there was a free space.
    pub fn push(&mut self, expression: Expression) -> bool {
        if self.lhs.is_none() {
            self.lhs = Some(Box::new(expression));
            true
        } else if self.rhs.is_none() {
            self.rhs = Some(Box::new(expression));
            true
        } else {
            false
        }
    }
}

impl Drop for Expression {
    fn drop(&mut self) {
        let mut list = vec![self.lhs.take(), self.rhs.take()];
        let mut cur_equation = list.pop();

        while let Some(equation) = cur_equation {
            if let Some(mut equation) = equation {
                list.push(equation.lhs.take());
                list.push(equation.rhs.take());
            }

            cur_equation = list.pop();
        }
    }
}

/// Extensions trait for [`Expression`].
pub trait ExpressionExtensions {
    /// Evaluates provided [`Expression`] against self.
    fn evaluate(&self, expression: &Expression) -> bool;
}

impl ExpressionExtensions for Vec<String> {
    /// Evaluates provided [`Expression`] against vector of strings.
    fn evaluate(&self, expression: &Expression) -> bool {
        evaluate(expression, self)
    }
}

impl ExpressionExtensions for Vec<&String> {
    /// Evaluates provided [`Expression`] against vector of string references.
    fn evaluate(&self, expression: &Expression) -> bool {
        evaluate(expression, self)
    }
}

/// Possible tokens for tokenized logical expression.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Token<'a> {
    And,
    Or,
    Not,
    Open,
    NotOpen,
    Close,
    Value(&'a str),
    NotValue(&'a str),
}

/// Parses vector of [`Token`]s and returns them as a [`Expression`].
fn parse_tokens(tokens: Vec<Token>) -> Expression {
    fn check_current(current: &mut Expression) {
        if current.is_complete() {
            let old = std::mem::take(current);
            current.push(old);
        }
    }

    let mut current = Expression::default();
    let mut stack = Vec::new();

    for t in tokens {
        match t {
            Token::And => {
                check_current(&mut current);
                current.op = Operator::And;
            },
            Token::Or => {
                check_current(&mut current);
                current.op = Operator::Or;
            },
            Token::Not => current.is_negation = true,
            Token::Open => stack.push(std::mem::take(&mut current)),
            Token::NotOpen => {
                stack.push(std::mem::take(&mut current));
                current.is_negation = true;
            },
            Token::Close => {
                if let Some(mut prev) = stack.pop() {
                    prev.push(std::mem::take(&mut current));
                    current = prev;
                }
            },
            Token::Value(s) => {
                check_current(&mut current);
                current.push(Expression::new(s, false));
            },
            Token::NotValue(s) => {
                check_current(&mut current);
                current.push(Expression::new(s, true));
            },
        }
    }

    while let Some(mut prev) = stack.pop() {
        prev.push(std::mem::take(&mut current));
        current = prev;
    }

    if current.is_pointless() {
        *std::mem::take(&mut current.lhs).unwrap()
    } else {
        current
    }
}

/// Tokenizes provided logical expression string.
fn tokenize(expression: &str) -> Result<Vec<Token>, ParserError> {
    let mut result = Vec::with_capacity(expression.len() / 2);

    let mut token_start = 0;
    let mut has_value = false;
    let mut has_close = false;
    let mut has_negation = false;
    let mut open_start = 0;
    let mut open_count = 0;

    for (index, char) in expression.chars().enumerate() {
        if let Some(token) = match char {
            '&' => Some(Token::And),
            '|' => Some(Token::Or),
            '!' => Some(Token::Not),
            '(' => Some(Token::Open),
            ')' => Some(Token::Close),
            ' ' => None,
            _ => {
                if has_close {
                    return Err(ParserError::ExpectedOperator(index));
                }

                has_value = true;
                None
            },
        } {
            if (token == Token::And || token == Token::Or || token == Token::Close) && !has_value && !has_close {
                return Err(ParserError::UnexpectedOperator(index));
            } else if (token == Token::Open || token == Token::Not) && (has_value || has_close) {
                return Err(ParserError::ExpectedOperator(index));
            } else if token == Token::Not && has_negation {
                return Err(ParserError::UnexpectedOperator(index));
            }

            update_brackets_count(token, index, &mut open_count, &mut open_start);
            if token == Token::Close && open_count < 0 {
                return Err(ParserError::UnexpectedClosingBracket(index));
            }

            if has_value && token_start != index {
                push_value(&mut result, &expression[token_start..index]);
            }

            push_token(&mut result, token);

            has_value = false;
            has_negation = token == Token::Not;
            has_close = token == Token::Close;
            token_start = index + 1;
        }
    }

    if open_count != 0 {
        return Err(ParserError::ExpectedClosingBracket(open_start));
    }

    if has_value {
        push_value(&mut result, &expression[token_start..]);
    }

    Ok(result)
}

fn update_brackets_count(token: Token, index: usize, open_count: &mut i32, open_start: &mut usize) {
    if token == Token::Open {
        *open_count += 1;
        if *open_count == 1 {
            *open_start = index;
        }
    } else if token == Token::Close {
        *open_count -= 1;
    }
}

fn push_token<'a>(tokens: &mut Vec<Token<'a>>, token: Token<'a>) {
    if token == Token::Open {
        if let Some(last) = tokens.last_mut() {
            if *last == Token::Not {
                *last = Token::NotOpen;
                return;
            }
        }
    }

    tokens.push(token);
}

fn push_value<'a>(tokens: &mut Vec<Token<'a>>, value: &'a str) {
    if let Some(last) = tokens.last_mut() {
        if *last == Token::Not {
            *last = Token::NotValue(value);
            return;
        }
    }

    tokens.push(Token::Value(value));
}

/// Holds currently processed [`Expression`] together with evaluated values for `lhs` and `rhs`.
struct CurrentExpression<'a> {
    pub expression: &'a Expression,
    pub lhs: Option<bool>,
    pub rhs: Option<bool>,
}

impl<'a> CurrentExpression<'a> {
    /// Creates new [`CurrentExpression`] instance.
    pub fn new(expression: &'a Expression) -> Self {
        Self {
            expression,
            lhs: None,
            rhs: None,
        }
    }

    /// Adds value to the first empty slot (`lhs` or `rhs`).
    pub fn push_value(&mut self, value: bool) {
        if self.lhs.is_none() {
            self.lhs = Some(value);
        } else if self.rhs.is_none() {
            self.rhs = Some(value);
        }
    }

    /// Returns value calculated from the `lhs` and `rhs` fields of the [`Expression`].
    pub fn value(&self) -> bool {
        if let Some(lhs) = self.lhs {
            if let Some(rhs) = self.rhs {
                let value = if self.expression.op == Operator::And {
                    lhs && rhs
                } else {
                    lhs || rhs
                };
                if self.expression.is_negation { !value } else { value }
            } else if self.expression.is_negation {
                !lhs
            } else {
                lhs
            }
        } else {
            false
        }
    }
}

/// Evaluates expression for specified statements.
fn evaluate<T: AsRef<str>>(expression: &Expression, statements: &[T]) -> bool {
    if expression.is_value() {
        return evaluate_value(expression, statements);
    }

    let mut stack = Vec::new();
    let mut maybe_current = Some(CurrentExpression::new(expression));

    while let Some(current) = maybe_current {
        if current.expression.is_pointless() {
            let new_expr = CurrentExpression::new(current.expression.lhs.as_ref().unwrap());
            maybe_current = Some(new_expr);
        } else if current.expression.lhs.is_some() && current.lhs.is_none() {
            let new_expr = CurrentExpression::new(current.expression.lhs.as_ref().unwrap());
            stack.push(current);
            maybe_current = Some(new_expr);
        } else if current.expression.rhs.is_some() && current.rhs.is_none() {
            let new_expr = CurrentExpression::new(current.expression.rhs.as_ref().unwrap());
            stack.push(current);
            maybe_current = Some(new_expr);
        } else if current.lhs.is_some() && (current.rhs.is_some() || current.expression.has_only_lhs()) {
            maybe_current = stack.pop();
            match maybe_current.as_mut() {
                Some(expr) => expr.push_value(current.value()),
                None => return current.value(),
            }
        } else if current.expression.is_value() {
            let value = evaluate_value(current.expression, statements);
            maybe_current = stack.pop();
            match maybe_current.as_mut() {
                Some(expr) => expr.push_value(value),
                None => return value,
            }
        } else {
            // we shouldn't get here
            break;
        }
    }

    false
}

fn evaluate_value<T: AsRef<str>>(expression: &Expression, statements: &[T]) -> bool {
    if let Some(value) = expression.value.as_deref() {
        if expression.is_negation {
            statements.iter().all(|s| !s.as_ref().contains(value))
        } else {
            statements.iter().any(|s| s.as_ref().contains(value))
        }
    } else {
        false
    }
}
