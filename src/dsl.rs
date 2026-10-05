//! Pure parsing and compilation of the Vapor Sentinel DSL.
//!
//! The DSL produces declarative notification requests only. It has no
//! filesystem or network side effects.

use crate::observation::Observation;
use anyhow::{bail, Context, Result};
use pest::Parser;
use pest_derive::Parser;

pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_NESTING_DEPTH: usize = 16;
const MAX_MESSAGES: usize = 64;
const MAX_MESSAGE_BYTES: usize = 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_LITERAL_VALUE: u64 = 1_000_000_000_000;

#[derive(Parser)]
#[grammar = "vapor.pest"]
struct VaporParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonOp {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl ComparisonOp {
    fn matches(&self, actual: u64, expected: u64) -> bool {
        match self {
            Self::GreaterThan => actual > expected,
            Self::GreaterThanOrEqual => actual >= expected,
            Self::LessThan => actual < expected,
            Self::LessThanOrEqual => actual <= expected,
            Self::Equal => actual == expected,
            Self::NotEqual => actual != expected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Metric(String),
    Comparison {
        metric: String,
        operator: ComparisonOp,
        value: u64,
    },
}

impl Condition {
    fn matches(&self, observation: &Observation) -> bool {
        match self {
            Self::Metric(metric) => observation.metric() == metric,
            Self::Comparison {
                metric,
                operator,
                value,
            } => observation.metric() == metric && operator.matches(observation.value(), *value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Send(String),
    If {
        condition: Condition,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    statements: Vec<Statement>,
}

impl Program {
    /// Bind deployment-controlled policies to the metrics the selected adapter emits.
    pub fn validate_metrics(&self, supported: &[&str]) -> Result<()> {
        fn visit(statements: &[Statement], supported: &[&str]) -> Result<()> {
            for statement in statements {
                if let Statement::If { condition, body } = statement {
                    let metric = match condition {
                        Condition::Metric(metric) | Condition::Comparison { metric, .. } => metric,
                    };
                    if !supported.contains(&metric.as_str()) {
                        bail!("unknown or unavailable policy metric: {metric}");
                    }
                    visit(body, supported)?;
                }
            }
            Ok(())
        }
        visit(&self.statements, supported)
    }

    pub fn requested_messages(&self, observation: &Observation) -> Vec<String> {
        fn collect(statements: &[Statement], observation: &Observation, output: &mut Vec<String>) {
            for statement in statements {
                match statement {
                    Statement::Send(message) => output.push(message.clone()),
                    Statement::If { condition, body } if condition.matches(observation) => {
                        collect(body, observation, output);
                    }
                    Statement::If { .. } => {}
                }
            }
        }

        let mut output = Vec::new();
        collect(&self.statements, observation, &mut output);
        output
    }
}

pub fn parse_program(source: &str) -> Result<Program> {
    if source.len() > MAX_SOURCE_BYTES {
        bail!("Vapor source exceeds {} bytes", MAX_SOURCE_BYTES);
    }

    check_structure(source)?;

    let root = VaporParser::parse(Rule::vapor_func, source)?
        .next()
        .context("Vapor source produced no root node")?;
    let body = root
        .into_inner()
        .find(|pair| pair.as_rule() == Rule::body)
        .context("Vapor source produced no body")?;

    let mut message_count = 0;
    let program = Program {
        statements: parse_body(body, 0, &mut message_count)?,
    };
    let mut bytes = 0;
    fn message_bytes(statements: &[Statement], bytes: &mut usize) {
        for statement in statements {
            match statement {
                Statement::Send(message) => *bytes += message.len() + 3,
                Statement::If { body, .. } => message_bytes(body, bytes),
            }
        }
    }
    message_bytes(&program.statements, &mut bytes);
    if bytes.saturating_sub(3) > crate::policy::MAX_NOTIFICATION_BYTES {
        bail!(
            "combined notification messages exceed {} bytes",
            crate::policy::MAX_NOTIFICATION_BYTES
        );
    }
    Ok(program)
}

/// Count braces before invoking the recursive parser. Strings are opaque and
/// escapes are unsupported, exactly as in the grammar. The function body adds
/// one level beyond MAX_NESTING_DEPTH. Memory and stack use are constant here.
fn check_structure(source: &str) -> Result<()> {
    let mut quoted = false;
    let mut depth = 0usize;
    for byte in source.bytes() {
        match byte {
            b'"' => quoted = !quoted,
            b'{' if !quoted => {
                depth += 1;
                if depth > MAX_NESTING_DEPTH + 1 {
                    bail!("Vapor nesting exceeds depth {}", MAX_NESTING_DEPTH);
                }
            }
            b'}' if !quoted => {
                depth = depth.checked_sub(1).context("unmatched closing brace")?;
            }
            _ => {}
        }
    }
    if quoted || depth != 0 {
        bail!("unterminated string or block");
    }
    Ok(())
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
                bail!(
                    "Vapor source exceeds {} notification messages",
                    MAX_MESSAGES
                );
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
            let condition =
                parse_condition(inner.next().context("if statement missing condition")?)?;

            let body = parse_body(
                inner.next().context("if statement missing body")?,
                depth + 1,
                message_count,
            )?;

            Ok(Statement::If { condition, body })
        }
        rule => bail!("unsupported Vapor rule: {rule:?}"),
    }
}

fn parse_condition(pair: pest::iterators::Pair<'_, Rule>) -> Result<Condition> {
    let mut inner = pair.into_inner();
    let first = inner.next().context("if condition is empty")?;

    match first.as_rule() {
        Rule::identifier => {
            let metric = first.as_str().to_owned();
            validate_identifier(&metric)?;
            Ok(Condition::Metric(metric))
        }
        Rule::comparison => {
            let mut comparison = first.into_inner();
            let metric = comparison
                .next()
                .context("comparison missing metric")?
                .as_str()
                .to_owned();
            validate_identifier(&metric)?;

            let operator = match comparison
                .next()
                .context("comparison missing operator")?
                .as_str()
            {
                ">" => ComparisonOp::GreaterThan,
                ">=" => ComparisonOp::GreaterThanOrEqual,
                "<" => ComparisonOp::LessThan,
                "<=" => ComparisonOp::LessThanOrEqual,
                "==" => ComparisonOp::Equal,
                "!=" => ComparisonOp::NotEqual,
                other => bail!("unsupported comparison operator: {other}"),
            };

            let value = comparison
                .next()
                .context("comparison missing numeric value")?
                .as_str()
                .parse::<u64>()
                .context("comparison value is not a valid unsigned integer")?;

            if value > MAX_LITERAL_VALUE {
                bail!("Vapor comparison literal exceeds {}", MAX_LITERAL_VALUE);
            }

            Ok(Condition::Comparison {
                metric,
                operator,
                value,
            })
        }
        rule => bail!("unsupported Vapor condition: {rule:?}"),
    }
}

fn validate_identifier(metric: &str) -> Result<()> {
    if metric.len() > MAX_IDENTIFIER_BYTES {
        bail!(
            "Vapor metric identifier exceeds {} bytes",
            MAX_IDENTIFIER_BYTES
        );
    }
    Ok(())
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
    if message.contains('\\') {
        bail!("Vapor notification message does not support escape sequences");
    }

    Ok(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    fn observation(value: u64) -> Observation {
        Observation::new("SYSTEM_USED_MEMORY_MIB", 1, value).unwrap()
    }

    #[test]
    fn parses_matching_metric_selector() -> Result<()> {
        let program = parse_program(
            r#"vapor sentinel() {
                if(SYSTEM_USED_MEMORY_MIB) {
                    send("memory alert");
                }
            }"#,
        )?;

        assert_eq!(
            program.requested_messages(&observation(150)),
            vec!["memory alert"]
        );
        assert!(program
            .requested_messages(&Observation::new("OTHER_METRIC", 1, 150).unwrap())
            .is_empty());
        Ok(())
    }

    #[test]
    fn comparisons_are_evaluated_against_observation_values() -> Result<()> {
        let program = parse_program(
            r#"vapor sentinel() {
                if(SYSTEM_USED_MEMORY_MIB >= 100) {
                    send("high");
                }
                if(SYSTEM_USED_MEMORY_MIB < 100) {
                    send("low");
                }
                if(SYSTEM_USED_MEMORY_MIB == 120) {
                    send("exact");
                }
            }"#,
        )?;

        assert_eq!(
            program.requested_messages(&observation(120)),
            vec!["high", "exact"]
        );
        assert_eq!(program.requested_messages(&observation(80)), vec!["low"]);
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
        let source = "vapor sentinel() { send(\"bad\0message\"); }";
        assert!(parse_program(source).is_err());
    }

    #[test]
    fn escaped_notification_syntax_is_rejected() {
        let source = r#"vapor sentinel() { send("bad\message"); }"#;
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
            "x".repeat(MAX_MESSAGE_BYTES + 1)
        );
        assert!(parse_program(&source).is_err());
    }

    #[test]
    fn oversized_comparison_literal_is_rejected() {
        let source = format!(
            "vapor sentinel() {{ if(SYSTEM_USED_MEMORY_MIB > {}) {{ send(\"too large\"); }} }}",
            MAX_LITERAL_VALUE + 1
        );
        assert!(parse_program(&source).is_err());
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn deeply_nested_source_is_rejected_before_recursive_parse() {
        let source = format!(
            "vapor s(){{{}send(\"deep\");{}}}",
            "if(M){".repeat(9000),
            "}".repeat(9000)
        );
        assert!(source.len() < MAX_SOURCE_BYTES);
        assert!(parse_program(&source)
            .unwrap_err()
            .to_string()
            .contains("nesting"));
    }
    #[test]
    fn exact_depth_limit_and_quoted_braces_work() {
        let source = format!(
            "vapor s(){{{}send(\"{{{{message}}}}\");{}}}",
            "if(M){".repeat(MAX_NESTING_DEPTH),
            "}".repeat(MAX_NESTING_DEPTH)
        );
        assert!(parse_program(&source).is_ok());
    }
    #[test]
    fn lexical_tokens_cannot_contain_whitespace() {
        for source in [
            "vapor s(){if(SYSTEM_ USED_MEMORY_MIB>=100){send(\"bad\");}}",
            "vapor s(){if(M>=1 00){send(\"bad\");}}",
        ] {
            assert!(parse_program(source).is_err());
        }
    }
    #[test]
    fn metric_binding_rejects_typos_and_unavailable_adapters() {
        let program =
            parse_program("vapor s(){if(SYSTEM_USED_MEMORRY_MIB>=100){send(\"bad\");}}").unwrap();
        assert!(program
            .validate_metrics(&["SYSTEM_USED_MEMORY_MIB"])
            .is_err());
    }
    #[test]
    fn aggregate_message_limit_is_enforced_at_load() {
        let source = format!(
            "vapor s(){{{}}}",
            format!("send(\"{}\");", "x".repeat(1024)).repeat(5)
        );
        assert!(parse_program(&source)
            .unwrap_err()
            .to_string()
            .contains("combined"));
        let source = format!(
            "vapor s(){{send(\"{}\");send(\"{}\");send(\"{}\");send(\"{}\");}}",
            "x".repeat(1024),
            "x".repeat(1024),
            "x".repeat(1024),
            "x".repeat(1015)
        );
        let program = parse_program(&source).unwrap();
        assert_eq!(
            program
                .requested_messages(&Observation::new("M", 1, 1).unwrap())
                .join(" | ")
                .len(),
            crate::policy::MAX_NOTIFICATION_BYTES
        );
    }
}
