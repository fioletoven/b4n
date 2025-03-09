use std::fmt::Debug;
use thiserror::Error;

#[cfg(test)]
#[path = "./logical_expressions.tests.rs"]
mod logical_expressions_tests;

/// Possible errors from expanding string with logical expression.
#[derive(Error, Debug)]
pub enum ParserError {
    /// Operator was expected at the `index` position.
    #[error("expected operator at index {0}")]
    ExpectedOperator(usize),

    /// Value was expected at the `index` position.
    #[error("expected value at index {0}")]
    ExpectedValue(usize),

    /// Closing bracket was not expected at the `index` position.
    #[error("unexpected closing bracket at index {0}")]
    UnexpectedClosingBracket(usize),

    /// Closing bracket was expected for opening bracket at the `index` position.
    #[error("expected closing bracket for opening bracket at index {0}")]
    ExpectedClosingBracket(usize),
}

/// Expands provided logical expression to the two-dimensional array of strings.  
/// First dimension is the OR part, second dimension is AND part of the expression.
pub fn expand(expression: &str) -> Result<Vec<Vec<String>>, ParserError> {
    let tokens = tokenize(expression)?;
    let parsed = parse(tokens);
    let expanded = parsed.expand();
    let expanded = expanded
        .iter()
        .map(|i| i.iter().map(|s| (*s).trim().to_owned()).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    Ok(expanded)
}

/// Validates if the provided logical expression can be parsed and expanded to the two-dimensional array of strings.
pub fn validate(expression: &str) -> Result<(), ParserError> {
    let _ = tokenize(expression)?;
    Ok(())
}

/// Possible tokens for tokenized logical expression.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Token<'a> {
    And,
    Or,
    Open,
    Close,
    Value(&'a str),
}

/// Possible operators for logical expression.
#[derive(Default, Debug, PartialEq)]
enum Operator {
    And,
    #[default]
    Or,
}

/// Parsed tokens united as a condition that has `lhs` and `rhs` parts joined with the operator.  
/// **Note** that a condition can be also represented as the end, single value.
#[derive(Default)]
struct Condition<'a> {
    pub lhs: Option<Box<Condition<'a>>>,
    pub rhs: Option<Box<Condition<'a>>>,
    pub op: Operator,
    pub value: Option<&'a str>,
}

impl<'a> Condition<'a> {
    /// Creates new [`Condition`] instance that is a single value.
    pub fn value(s: &'a str) -> Self {
        Self {
            value: Some(s),
            ..Default::default()
        }
    }

    /// Returns `true` if [`Condition`] is complete: has both hand sides of the equation or is a value.
    pub fn is_complete(&self) -> bool {
        self.value.is_some() || (self.lhs.is_some() && self.rhs.is_some())
    }

    /// Pushes new [`Condition`] to the first empty hand side (left, then right).  
    /// **Note** that it returns `true` if there was a free space.
    pub fn push(&mut self, condition: Condition<'a>) -> bool {
        if self.lhs.is_none() {
            self.lhs = Some(Box::new(condition));
            true
        } else if self.rhs.is_none() {
            self.rhs = Some(Box::new(condition));
            true
        } else {
            false
        }
    }

    /// Expands [`Condition`] to the two-dimensional array of strings.  
    /// First dimension is the OR part, second dimension is AND part of the expression.
    pub fn expand(&self) -> Vec<Vec<&'a str>> {
        if let Some(value) = self.value {
            return vec![vec![value]];
        }

        match &self.op {
            Operator::And => self.expand_and(),
            Operator::Or => self.expand_or(),
        }
    }

    fn expand_and(&self) -> Vec<Vec<&'a str>> {
        let Some(lhs) = &self.lhs else {
            return vec![Vec::new()];
        };
        let Some(rhs) = &self.rhs else {
            return lhs.expand();
        };

        let lhs = lhs.expand();
        let rhs = rhs.expand();

        let mut result = Vec::new();
        for l in lhs {
            for r in &rhs {
                let mut n = Vec::new();
                n.extend(l.clone());
                n.extend(r.clone());
                result.push(n);
            }
        }

        result
    }

    fn expand_or(&self) -> Vec<Vec<&'a str>> {
        let Some(lhs) = &self.lhs else {
            return vec![Vec::new()];
        };
        let Some(rhs) = &self.rhs else {
            return lhs.expand();
        };

        let lhs = lhs.expand();
        let rhs = rhs.expand();

        let mut result = Vec::new();
        for l in lhs {
            result.push(l);
        }

        for r in rhs {
            result.push(r);
        }

        result
    }
}

/// Parses vector of [`Token`]s and returns them as a [`Condition`].
fn parse(tokens: Vec<Token>) -> Condition {
    let mut current = Condition::default();
    let mut stack = Vec::new();

    for t in tokens {
        match t {
            Token::And => {
                if current.is_complete() {
                    let old = std::mem::take(&mut current);
                    current.push(old);
                }

                current.op = Operator::And;
            }
            Token::Or => {
                if current.is_complete() {
                    let old = std::mem::take(&mut current);
                    current.push(old);
                }

                current.op = Operator::Or;
            }
            Token::Open => stack.push(std::mem::take(&mut current)),
            Token::Close => {
                if let Some(mut prev) = stack.pop() {
                    prev.push(std::mem::take(&mut current));
                    current = prev;
                }
            }
            Token::Value(s) => {
                if current.is_complete() {
                    let old = std::mem::take(&mut current);
                    current.push(old);
                }

                current.push(Condition::value(s));
            }
        }
    }

    while let Some(mut prev) = stack.pop() {
        prev.push(std::mem::take(&mut current));
        current = prev;
    }

    current
}

/// Tokenizes provided logical expression string.
fn tokenize(expression: &str) -> Result<Vec<Token>, ParserError> {
    let mut result = Vec::with_capacity(expression.len() / 2);

    let mut token_start = 0;
    let mut has_value = false;
    let mut has_close = false;
    let mut open_start = 0;
    let mut open_count = 0;

    for (index, char) in expression.chars().enumerate() {
        if let Some(token) = match char {
            '+' | '&' => Some(Token::And),
            '|' => Some(Token::Or),
            '(' => Some(Token::Open),
            ')' => Some(Token::Close),
            ' ' => None,
            _ => {
                if has_close {
                    return Err(ParserError::ExpectedOperator(index));
                }

                has_value = true;
                None
            }
        } {
            if (token == Token::And || token == Token::Or || token == Token::Close) && !has_value && !has_close {
                return Err(ParserError::ExpectedValue(index));
            } else if token == Token::Open && has_value {
                return Err(ParserError::ExpectedOperator(index));
            }

            has_close = false;
            if token == Token::Open {
                open_count += 1;
                if open_count == 1 {
                    open_start = index;
                }
            } else if token == Token::Close {
                has_close = true;
                open_count -= 1;
                if open_count < 0 {
                    return Err(ParserError::UnexpectedClosingBracket(index));
                }
            }

            if token_start != index && has_value {
                result.push(Token::Value(&expression[token_start..index]));
            }

            result.push(token);
            has_value = false;
            token_start = index + 1;
        }
    }

    if open_count != 0 {
        return Err(ParserError::ExpectedClosingBracket(open_start));
    }

    if has_value {
        result.push(Token::Value(&expression[token_start..]));
    }

    Ok(result)
}

/// Adds comparison extensions for two-dimensional arrays of strings.  
/// **Note** that first dimension is the `OR` part and second dimension is the `AND` part.
pub trait MatchExtensions {
    /// Returns `true` if the provided two-dimensional array is matching this object.
    fn is_matching(&self, expression: &[Vec<String>]) -> bool;
}

impl MatchExtensions for String {
    fn is_matching(&self, expression: &[Vec<String>]) -> bool {
        expression.iter().any(|e| e.iter().all(|i| self.contains(i)))
    }
}
