use crate::error::PolicyError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Key(String),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

/// Parse a simple policy expression language: `A`, `&&`, `||`, parentheses.
pub fn parse_expression(input: &str) -> Result<Expr, PolicyError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_or()?;
    if parser.has_more() {
        return Err(PolicyError::InvalidExpression(format!(
            "unexpected token '{}'",
            parser.peek().unwrap_or(&"".to_string())
        )));
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<String>,
    pos: usize,
}

impl Parser {
    fn has_more(&self) -> bool {
        self.pos < self.tokens.len()
    }

    fn peek(&self) -> Option<&String> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<String> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: &str) -> Result<(), PolicyError> {
        match self.consume() {
            Some(token) if token == expected => Ok(()),
            Some(token) => Err(PolicyError::InvalidExpression(format!(
                "expected '{expected}', found '{token}'"
            ))),
            None => Err(PolicyError::InvalidExpression(format!(
                "expected '{expected}', found end of input"
            ))),
        }
    }

    fn parse_or(&mut self) -> Result<Expr, PolicyError> {
        let mut left = self.parse_and()?;
        while self.peek().is_some_and(|t| t == "||") {
            self.consume();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, PolicyError> {
        let mut left = self.parse_factor()?;
        while self.peek().is_some_and(|t| t == "&&") {
            self.consume();
            let right = self.parse_factor()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, PolicyError> {
        match self.peek() {
            Some(token) if token == "(" => {
                self.consume();
                let expr = self.parse_or()?;
                self.expect(")")?;
                Ok(expr)
            }
            Some(token) if is_identifier(token) => {
                let id = self.consume().unwrap();
                Ok(Expr::Key(id))
            }
            Some(token) => Err(PolicyError::InvalidExpression(format!(
                "unexpected token '{token}'"
            ))),
            None => Err(PolicyError::InvalidExpression(
                "unexpected end of input".into(),
            )),
        }
    }
}

fn is_identifier(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn tokenize(input: &str) -> Result<Vec<String>, PolicyError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '(' || ch == ')' {
            tokens.push(ch.to_string());
            chars.next();
            continue;
        }

        if ch == '&' {
            chars.next();
            match chars.next() {
                Some('&') => tokens.push("&&".to_string()),
                _ => {
                    return Err(PolicyError::InvalidExpression(
                        "expected '&&' operator".into(),
                    ))
                }
            }
            continue;
        }

        if ch == '|' {
            chars.next();
            match chars.next() {
                Some('|') => tokens.push("||".to_string()),
                _ => {
                    return Err(PolicyError::InvalidExpression(
                        "expected '||' operator".into(),
                    ))
                }
            }
            continue;
        }

        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            let mut ident = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    ident.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(ident);
            continue;
        }

        return Err(PolicyError::InvalidExpression(format!(
            "unexpected character '{ch}'"
        )));
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_abc_primary() {
        let expr = parse_expression("(A && B) || (A && C)").unwrap();
        assert!(matches!(expr, Expr::Or(_, _)));
    }

    #[test]
    fn parses_simple_and() {
        let expr = parse_expression("A && B").unwrap();
        assert!(matches!(expr, Expr::And(_, _)));
    }
}
