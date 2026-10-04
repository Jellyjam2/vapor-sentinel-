//! Pure parsing and compilation of the Vapor Sentinel DSL.
//!
//! The DSL produces declarative notification requests only. It has no
//! filesystem or network side effects.

use anyhow::{bail, Context, Result};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "vapor.pest"]
pub struct VaporParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Send(String),
    IfMetric {
        metric: String,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    statements: Vec<Statement>,
}

impl Program {
    pub fn requested_messages(&self, metric: &str) -> Vec<String> {
        fn collect(statements: &[Statement], metric: &str, output: &mut Vec<String>) {
            for statement in statements {
                match statement {
                    Statement::Send(message) => output.push(message.clone()),
                    Statement::IfMetric { metric: name, body } if name == metric => {
                        collect(body, metric, output);
                    }
                    Statement::IfMetric { .. } => {}
                }
            }
        }

        let mut output = Vec::new();
        collect(&self.statements, metric, &mut output);
        output
    }
}

pub fn parse_program(source: &str) -> Result<Program> {
    let root = VaporParser::parse(Rule::vapor_func, source)?
        .next()
        .context("Vapor source produced no root node")?;
    let body = root
        .into_inner()
        .find(|pair| pair.as_rule() == Rule::body)
        .context("Vapor source produced no body")?;

    Ok(Program {
        statements: parse_body(body)?,
    })
}

fn parse_body(pair: pest::iterators::Pair<'_, Rule>) -> Result<Vec<Statement>> {
    pair.into_inner().map(parse_statement).collect()
}

fn parse_statement(pair: pest::iterators::Pair<'_, Rule>) -> Result<Statement> {
    match pair.as_rule() {
        Rule::send_stmt => {
            let message = pair
                .into_inner()
                .next()
                .context("send statement missing message")?;
            Ok(Statement::Send(unquote(message.as_str())?))
        }
        Rule::if_stmt => {
            let mut inner = pair.into_inner();
            let metric = inner
                .next()
                .context("if statement missing metric")?
                .as_str()
                .to_owned();
            let body = parse_body(
                inner
                    .next()
                    .context("if statement missing body")?,
            )?;
            Ok(Statement::IfMetric { metric, body })
        }
        rule => bail!("unsupported Vapor rule: {rule:?}"),
    }
}

fn unquote(value: &str) -> Result<String> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        bail!("invalid Vapor string literal");
    }
    Ok(value[1..value.len() - 1].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_notification_inside_matching_metric_rule() {
        let program = parse_program(
            r#"vapor sentinel() {
                if(SYSTEM_USED_MEMORY_MB) {
                    send("memory threshold breached");
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            program.requested_messages("SYSTEM_USED_MEMORY_MB"),
            vec!["memory threshold breached"]
        );
        assert!(program.requested_messages("OTHER_METRIC").is_empty());
    }

    #[test]
    fn unsupported_loop_is_rejected() {
        let source = r#"vapor sentinel() {
            while(SYSTEM_USED_MEMORY_MB) {
                send("unsupported");
            }
        }"#;

        assert!(parse_program(source).is_err());
    }

    #[test]
    fn unsupported_generic_statement_is_rejected() {
        let source = r#"vapor sentinel() {
            execute();
        }"#;

        assert!(parse_program(source).is_err());
    }

    #[test]
    fn assignment_syntax_is_rejected() {
        let source = r#"vapor sentinel() {
            amount = 1000;
        }"#;

        assert!(parse_program(source).is_err());
    }
}
