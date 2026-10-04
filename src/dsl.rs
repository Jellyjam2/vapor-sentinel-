//! Pure parsing and compilation of the Vapor Sentinel DSL.
//!
//! The DSL produces declarative notification requests only. It has no
//! filesystem or network side effects.

use anyhow::{bail, Context, Result};
use pest::Parser;
use pest_derive::Parser;

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_NESTING_DEPTH: usize = 16;
const MAX_MESSAGES: usize = 64;
const MAX_MESSAGE_BYTES: usize = 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;

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
    if source.len() > MAX_SOURCE_BYTES {
        bail!("Vapor source exceeds {} bytes", MAX_SOURCE_BYTES);
    }

    let root = VaporParser::parse(Rule::vapor_func, source)?
        .next()
        .context("Vapor source produced no root node")?;
    let body = root
        .into_inner()
        .find(|pair| pair.as_rule() == Rule::body)
        .context("Vapor source produced no body")?;

    let mut message_count = 0;
    Ok(Program {
        statements: parse_body(body, 0, &mut message_count)?,
    })
}

fn parse_body(
    pair: pest::iterators::Pair<'_, Rule>,
    depth: usize,
    message_count: &mut usize,
) -> Result<Vec<Statement>> {
    pair.into_inner()
        .map(|pair| parse_statement(pair, depth, message_count))
        .collect()
}

fn parse_statement(
    pair: pest::iterators::Pair<'_, Rule>,
    depth: usize,
    message_count: &mut usize,
) -> Result<Statement> {
    match pair.as_rule() {
        Rule::send_stmt => {
            *message_count = message_count
                .checked_add(1)
                .context("Vapor message count overflowed")?;
            if *message_count > MAX_MESSAGES {
                bail!("Vapor source exceeds {} notification messages", MAX_MESSAGES);
            }

            let message = pair
                .into_inner()
                .next()
                .context("send statement missing message")?;
            Ok(Statement::Send(unquote(message.as_str())?))
        }
        Rule::if_stmt => {
            if depth >= MAX_NESTING_DEPTH {
                bail!("Vapor nesting exceeds depth {}", MAX_NESTING_DEPTH);
            }

            let mut inner = pair.into_inner();
            let metric = inner
                .next()
                .context("if statement missing metric")?
                .as_str()
                .to_owned();

            if metric.len() > MAX_IDENTIFIER_BYTES {
                bail!("Vapor metric identifier exceeds {} bytes", MAX_IDENTIFIER_BYTES);
            }

            let body = parse_body(
                inner.next().context("if statement missing body")?,
                depth + 1,
                message_count,
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

    let message = &value[1..value.len() - 1];
    if message.is_empty() {
        bail!("Vapor notification message cannot be empty");
    }
    if message.len() > MAX_MESSAGE_BYTES {
        bail!(
            "Vapor notification message exceeds {} bytes",
            MAX_MESSAGE_BYTES
        );
    }
    if message.chars().any(char::is_control) {
        bail!("Vapor notification message contains control characters");
    }

    Ok(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_notification_inside_matching_metric_rule() -> anyhow::Result<()> {
        let program = parse_program(
            r#"vapor sentinel() {
                if(SYSTEM_USED_MEMORY_MIB) {
                    send("memory threshold breached");
                }
            }"#,
        )?;

        assert_eq!(
            program.requested_messages("SYSTEM_USED_MEMORY_MIB"),
            vec!["memory threshold breached"]
        );
        assert!(program.requested_messages("OTHER_METRIC").is_empty());
        Ok(())
    }

    #[test]
    fn unsupported_loop_is_rejected() {
        let source = r#"vapor sentinel() {
            while(SYSTEM_USED_MEMORY_MIB) {
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

    #[test]
    fn empty_notification_message_is_rejected() {
        let source = r#"vapor sentinel() {
            send("");
        }"#;

        assert!(parse_program(source).is_err());
    }

    #[test]
    fn control_character_in_notification_message_is_rejected() {
        let source = "vapor sentinel() { send(\"bad\\u{0000}message\"); }";
        assert!(parse_program(source).is_err());
    }

    #[test]
    fn deeply_nested_program_is_rejected() {
        let mut source = String::from("vapor sentinel() { ");
        for _ in 0..=MAX_NESTING_DEPTH {
            source.push_str("if(SYSTEM_USED_MEMORY_MIB){");
        }
        source.push_str("send(\"too deep\");");
        for _ in 0..=MAX_NESTING_DEPTH {
            source.push('}');
        }

        assert!(parse_program(&source).is_err());
    }

    #[test]
    fn oversized_source_is_rejected() {
        let source = format!(
            "vapor sentinel() {{ send(\"{}\"); }}",
            "x".repeat(MAX_SOURCE_BYTES)
        );
        assert!(parse_program(&source).is_err());
    }

    #[test]
    fn too_many_messages_are_rejected() {
        let mut source = String::from("vapor sentinel() { ");
        for _ in 0..=MAX_MESSAGES {
            source.push_str("send(\"alert\");");
        }
        source.push('}');

        assert!(parse_program(&source).is_err());
    }

    #[test]
    fn oversized_message_is_rejected() {
        let source = format!(
            "vapor sentinel() {{ send(\"{}\"); }}",
            "x".repeat(MAX_MESSAGE_BYTES)
        );
        assert!(parse_program(&source).is_err());
    }
}
