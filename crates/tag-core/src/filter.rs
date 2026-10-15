//! Compiled Mp3tag filter expressions, based on https://docs.mp3tag.de/filter/.
//! Queries, scripting templates, and regular expressions are parsed once.
//! Evaluation operates only on supplied metadata snapshots and never reads files.
use crate::script::{context_values, regex, CompiledTemplate, EvalContext};
use crate::Fields;
use regex_compat::Regex;
use std::borrow::Cow;

const MAX_QUERY_BYTES: usize = 1_048_576;
const MAX_TOKENS: usize = 8192;
const MAX_DEPTH: usize = 128;

#[derive(Debug)]
pub struct Filter {
    expression: Expression,
}

impl Filter {
    pub fn matches(&self, fields: &Fields) -> Result<bool, String> {
        self.matches_with_context(&EvalContext::new(fields))
    }

    pub fn matches_with_context(&self, context: &EvalContext<'_>) -> Result<bool, String> {
        self.expression.matches(&context.snapshot_clock())
    }
}

pub fn compile_filter(query: &str) -> Result<Filter, String> {
    if query.len() > MAX_QUERY_BYTES {
        return Err("Filter query length limit exceeded".into());
    }
    if query.trim().is_empty() {
        return Ok(Filter {
            expression: Expression::All,
        });
    }
    let tokens = tokenize(query)?;
    let filter_keywords = [
        "AND", "OR", "NOT", "HAS", "IS", "GREATER", "LESS", "EQUAL", "PRESENT", "MISSING",
        "ABSENT", "MATCHES", "MATCH",
    ];
    // Ordinary searches treat punctuation as literal word content. Parentheses
    // only group expressions when the query actually contains filter syntax.
    if !tokens
        .iter()
        .any(|token| filter_keywords.iter().any(|word| token.keyword(word)))
    {
        let text = if tokens.len() == 1 {
            match &tokens[0].kind {
                TokenKind::Text(text, true) => text.as_str(),
                _ => query.trim(),
            }
        } else {
            if tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Text(_, true)))
            {
                return Err("Enclose the complete search string in one pair of quotes".into());
            }
            query.trim()
        };
        return Ok(Filter {
            expression: Expression::Search(words(text)),
        });
    }
    let mut parser = Parser {
        query,
        tokens,
        position: 0,
    };
    let expression = parser.or(0)?;
    if parser.position != parser.tokens.len() {
        return Err(format!(
            "Unexpected filter token at position {}",
            parser.tokens[parser.position].start + 1
        ));
    }
    Ok(Filter { expression })
}

pub fn matches_filter(query: &str, fields: &Fields) -> Result<bool, String> {
    compile_filter(query)?.matches(fields)
}

#[derive(Debug)]
enum Expression {
    All,
    Search(Vec<String>),
    Predicate(Selector, Operation),
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Not(Box<Expression>),
}

#[derive(Debug)]
enum Selector {
    Field(String),
    Template(CompiledTemplate),
    AllFields,
}

#[derive(Debug)]
enum Operation {
    Has(Vec<String>),
    Is(String),
    Numeric(Comparison, i64),
    Present(bool),
    Matches(Regex),
}

#[derive(Debug)]
enum Comparison {
    Equal,
    Greater,
    Less,
}

fn values<'a>(fields: &'a Fields, name: &str) -> Option<&'a Vec<String>> {
    fields.get(name).or_else(|| {
        fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, values)| values)
    })
}

fn tag_values(fields: &Fields) -> impl Iterator<Item = &String> {
    fields
        .iter()
        .filter(|(name, _)| !name.starts_with('_'))
        .flat_map(|(_, values)| values)
}

fn searchable_values(fields: &Fields) -> Vec<Cow<'_, str>> {
    let mut searchable: Vec<Cow<'_, str>> = tag_values(fields)
        .map(|value| Cow::Borrowed(value.as_str()))
        .collect();
    if let Some(filename) = values(fields, "_FILENAME_EXT").or_else(|| values(fields, "_FILENAME"))
    {
        searchable.extend(filename.iter().map(|value| Cow::Borrowed(value.as_str())));
    }
    searchable
}

fn words(text: &str) -> Vec<String> {
    text.split(' ')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn contains_words(values: &[Cow<'_, str>], words: &[String]) -> bool {
    if values.is_empty() {
        return false;
    }
    let normalized: Vec<String> = values.iter().map(|value| value.to_lowercase()).collect();
    words
        .iter()
        .all(|word| normalized.iter().any(|value| value.contains(word)))
}

impl Selector {
    fn values<'a>(&self, context: &'a EvalContext<'_>) -> Result<Vec<Cow<'a, str>>, String> {
        let fields = context.fields;
        match self {
            Self::Field(name)
                if ["_DATE", "_DATETIME", "_WORKINGDIR", "_WORKINGPATH"]
                    .iter()
                    .any(|context_name| name.eq_ignore_ascii_case(context_name)) =>
            {
                Ok(match context_values(context, name)? {
                    Some(Cow::Borrowed(values)) => values
                        .iter()
                        .map(|value| Cow::Borrowed(value.as_str()))
                        .collect(),
                    Some(Cow::Owned(values)) => values.into_iter().map(Cow::Owned).collect(),
                    None => Vec::new(),
                })
            }
            Self::Field(name) => Ok(values(fields, name)
                .into_iter()
                .flatten()
                .map(|value| Cow::Borrowed(value.as_str()))
                .collect()),
            Self::Template(template) => {
                Ok(vec![Cow::Owned(template.evaluate_with_context(context)?)])
            }
            Self::AllFields => Ok(tag_values(fields)
                .map(|value| Cow::Borrowed(value.as_str()))
                .collect()),
        }
    }
}

impl Expression {
    fn matches(&self, context: &EvalContext<'_>) -> Result<bool, String> {
        let fields = context.fields;
        match self {
            Self::All => Ok(true),
            Self::Search(terms) => Ok(contains_words(&searchable_values(fields), terms)),
            Self::Not(expression) => Ok(!expression.matches(context)?),
            Self::And(expressions) => {
                for expression in expressions {
                    if !expression.matches(context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Or(expressions) => {
                for expression in expressions {
                    if expression.matches(context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Self::Predicate(selector, operation) => {
                let candidates = selector.values(context)?;
                match operation {
                    Operation::Has(terms) => Ok(contains_words(&candidates, terms)),
                    Operation::Is(expected) => Ok(candidates
                        .iter()
                        .any(|value| value.to_lowercase() == *expected)),
                    Operation::Present(wanted) => Ok(candidates.is_empty() != *wanted),
                    Operation::Matches(expression) => {
                        for value in &candidates {
                            if expression.is_match(value)? {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                    Operation::Numeric(comparison, expected) => Ok(candidates
                        .iter()
                        .filter_map(|value| leading_integer(value))
                        .any(|number| match comparison {
                            Comparison::Equal => number == *expected,
                            Comparison::Greater => number > *expected,
                            Comparison::Less => number < *expected,
                        })),
                }
            }
        }
    }
}

/// Numeric part of a field value: the leading integer, ignoring whatever
/// follows it, so `3/12` compares as 3 and `2024-05-01` as 2024. Values that
/// do not start with a number take no part in numeric comparisons.
fn leading_integer(value: &str) -> Option<i64> {
    let value = value.trim_start();
    let digits = value.strip_prefix(['-', '+']).unwrap_or(value);
    let length = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if length == 0 {
        return None;
    }
    let number = value[..value.len() - digits.len() + length].parse::<i64>();
    // Overlong digit runs saturate rather than dropping out of the comparison.
    Some(number.unwrap_or(if value.starts_with('-') {
        i64::MIN
    } else {
        i64::MAX
    }))
}

#[derive(Debug)]
enum TokenKind {
    Text(String, bool),
    Open,
    Close,
}

#[derive(Debug)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

impl Token {
    fn keyword(&self, keyword: &str) -> bool {
        matches!(&self.kind, TokenKind::Text(text, false) if text == keyword)
    }
}

fn tokenize(query: &str) -> Result<Vec<Token>, String> {
    let mut characters = query.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some((start, character)) = characters.next() {
        if character.is_whitespace() {
            continue;
        }
        let kind = match character {
            '(' => TokenKind::Open,
            ')' => TokenKind::Close,
            '"' => {
                let mut text = String::new();
                loop {
                    let (_, next) = characters.next().ok_or("Unterminated filter quotation")?;
                    if next == '"' {
                        break;
                    }
                    text.push(next);
                }
                TokenKind::Text(text, true)
            }
            _ => {
                let mut text = String::from(character);
                let mut in_class = character == '[';
                if character == '\\' {
                    if let Some((_, next)) = characters.next() {
                        text.push(next);
                    }
                }
                while let Some((_, next)) = characters.peek() {
                    if !in_class && (next.is_whitespace() || matches!(next, '(' | ')' | '"')) {
                        break;
                    }
                    let (_, next) = characters.next().unwrap();
                    text.push(next);
                    if next == '\\' {
                        if let Some((_, escaped)) = characters.next() {
                            text.push(escaped);
                        }
                    } else if next == '[' {
                        in_class = true;
                    } else if next == ']' {
                        in_class = false;
                    }
                }
                TokenKind::Text(text, false)
            }
        };
        let end = characters
            .peek()
            .map_or(query.len(), |(position, _)| *position);
        tokens.push(Token { kind, start, end });
        if tokens.len() > MAX_TOKENS {
            return Err("Filter token limit exceeded".into());
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    query: &'a str,
    tokens: Vec<Token>,
    position: usize,
}

impl Parser<'_> {
    fn keyword(&self, word: &str) -> bool {
        self.tokens
            .get(self.position)
            .is_some_and(|token| token.keyword(word))
    }

    fn or(&mut self, depth: usize) -> Result<Expression, String> {
        let mut expressions = vec![self.and(depth)?];
        while self.keyword("OR") {
            self.position += 1;
            expressions.push(self.and(depth)?);
        }
        Ok(if expressions.len() == 1 {
            expressions.pop().unwrap()
        } else {
            Expression::Or(expressions)
        })
    }

    fn and(&mut self, depth: usize) -> Result<Expression, String> {
        let mut expressions = vec![self.unary(depth)?];
        while self.keyword("AND") {
            self.position += 1;
            expressions.push(self.unary(depth)?);
        }
        Ok(if expressions.len() == 1 {
            expressions.pop().unwrap()
        } else {
            Expression::And(expressions)
        })
    }

    fn unary(&mut self, depth: usize) -> Result<Expression, String> {
        if depth > MAX_DEPTH {
            return Err("Filter nesting limit exceeded".into());
        }
        if self.keyword("NOT") {
            self.position += 1;
            return Ok(Expression::Not(Box::new(self.unary(depth + 1)?)));
        }
        if self
            .tokens
            .get(self.position)
            .is_some_and(|token| matches!(token.kind, TokenKind::Open))
        {
            self.position += 1;
            let inner = self.or(depth + 1)?;
            if !self
                .tokens
                .get(self.position)
                .is_some_and(|token| matches!(token.kind, TokenKind::Close))
            {
                return Err("Unclosed filter group".into());
            }
            self.position += 1;
            return Ok(inner);
        }
        self.predicate()
    }

    fn text(&self, start: usize, end: usize) -> Result<String, String> {
        if start == end {
            return Err("Missing filter operand".into());
        }
        if end == start + 1 {
            if let TokenKind::Text(text, _) = &self.tokens[start].kind {
                return Ok(text.clone());
            }
        }
        if self.tokens[start..end]
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Text(_, true)))
        {
            return Err("Enclose the complete filter operand in one pair of quotes".into());
        }
        Ok(self.query[self.tokens[start].start..self.tokens[end - 1].end].to_owned())
    }

    fn rhs(&mut self) -> Result<String, String> {
        let start = self.position;
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.position) {
            match token.kind {
                TokenKind::Open => {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err("Filter operand nesting limit exceeded".into());
                    }
                }
                TokenKind::Close if depth == 0 => break,
                TokenKind::Close => depth -= 1,
                _ if depth == 0 && (token.keyword("AND") || token.keyword("OR")) => break,
                _ => {}
            }
            self.position += 1;
        }
        if depth != 0 {
            return Err("Unclosed parentheses in filter operand".into());
        }
        self.text(start, self.position)
    }

    fn predicate(&mut self) -> Result<Expression, String> {
        let start = self.position;
        let keywords = [
            "HAS", "IS", "GREATER", "LESS", "EQUAL", "PRESENT", "MISSING", "ABSENT", "MATCHES",
        ];
        while let Some(token) = self.tokens.get(self.position) {
            if token.keyword("MATCH") {
                return Err("Unsupported filter operator MATCH; Mp3tag uses MATCHES".into());
            }
            if keywords.iter().any(|keyword| token.keyword(keyword)) {
                break;
            }
            if token.keyword("AND")
                || token.keyword("OR")
                || matches!(token.kind, TokenKind::Close | TokenKind::Open)
            {
                break;
            }
            self.position += 1;
        }
        let lhs = self.text(start, self.position)?;
        let Some(operator) = self
            .tokens
            .get(self.position)
            .and_then(|token| {
                if let TokenKind::Text(text, false) = &token.kind {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .filter(|word| keywords.contains(&word.as_str()))
        else {
            return Ok(Expression::Search(words(&lhs)));
        };
        self.position += 1;
        let presence = ["PRESENT", "MISSING", "ABSENT"].contains(&operator.as_str());
        let selector = if lhs == "*" {
            if !["HAS", "MATCHES"].contains(&operator.as_str()) {
                return Err("The all-fields selector is supported only for HAS and MATCHES".into());
            }
            Selector::AllFields
        } else if presence {
            // Mp3tag also accepts a simple %field% spelling for presence tests.
            let name = if lhs.len() >= 2
                && lhs.starts_with('%')
                && lhs.ends_with('%')
                && lhs[1..lhs.len() - 1].find('%').is_none()
            {
                &lhs[1..lhs.len() - 1]
            } else {
                &lhs
            };
            if name.is_empty() || name.contains(['$', '%']) {
                return Err(
                    "Presence filters require a field name, not a scripting expression".into(),
                );
            }
            Selector::Field(name.to_ascii_uppercase())
        } else if lhs.contains(['$', '%']) {
            Selector::Template(CompiledTemplate::compile(&lhs)?)
        } else {
            Selector::Field(lhs.to_ascii_uppercase())
        };
        let operation = if presence {
            Operation::Present(operator == "PRESENT")
        } else {
            let rhs = self.rhs()?;
            match operator.as_str() {
                "HAS" => Operation::Has(words(&rhs)),
                "IS" => Operation::Is(rhs.to_lowercase()),
                "MATCHES" => Operation::Matches(regex(&rhs, true)?),
                "GREATER" | "LESS" | "EQUAL" => {
                    let integer = rhs.parse::<i64>().map_err(|_| "Numeric filters require an integer operand; durations and decimals are not documented filter numbers")?;
                    let comparison = match operator.as_str() {
                        "GREATER" => Comparison::Greater,
                        "LESS" => Comparison::Less,
                        _ => Comparison::Equal,
                    };
                    Operation::Numeric(comparison, integer)
                }
                _ => unreachable!(),
            }
        };
        Ok(Expression::Predicate(selector, operation))
    }
}
