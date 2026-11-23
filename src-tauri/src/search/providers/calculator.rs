//! Pure expression calculator. No shell, no eval, no I/O.

use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchResult, SearchResultAction, SearchResultKind,
};
use std::time::Instant;

/// Raw, untrimmed character count.
pub(crate) const MAX_CALCULATOR_QUERY_CHARS: usize = 256;
/// Shared by parentheses and stacked prefix operators.
pub(crate) const MAX_CALCULATOR_NESTING_DEPTH: usize = 32;
pub(crate) const CALCULATOR_RESULT_ID: &str = "calculator:result";
const MAX_EXACT_INTEGER: i128 = 1 << 53;
/// Below this an integral float shows the same digits as the integer, so no rounding is hidden.
const MAX_FLOAT_AS_INTEGER: f64 = 1e12;
const SIGNIFICANT_DIGITS: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalculatorValue {
    pub(crate) text: String,
    pub(crate) hex: Option<String>,
    pub(crate) bin: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CalculatorSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_calculator(query: &str, limit: usize) -> CalculatorSearchRun {
    let started_at = iso_now();
    let started = Instant::now();
    let mut results = evaluate_calculator_query(query)
        .map(calculator_result)
        .into_iter()
        .collect::<Vec<_>>();
    results.truncate(limit);

    CalculatorSearchRun {
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Calculator,
            started_at,
            ended_at: Some(iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: SearchProviderCacheState::Hit,
            cache_age_ms: None,
            result_count: results.len(),
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::Calculator,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: None,
        },
        results,
    }
}

fn calculator_result(value: CalculatorValue) -> SearchResult {
    let subtitle = match (&value.hex, &value.bin) {
        (Some(hex), Some(bin)) => format!("{hex} - {bin}"),
        _ => "Calculator".to_string(),
    };
    SearchResult {
        // The answer is part of the identity, so a stale activation cannot copy a newer answer.
        record_key: format!("{CALCULATOR_RESULT_ID}:{}", value.text),
        id: CALCULATOR_RESULT_ID.to_string(),
        provider_id: SearchProviderId::Calculator,
        kind: SearchResultKind::Calculator,
        title: value.text.clone(),
        subtitle: Some(subtitle),
        path: None,
        action: SearchResultAction::CopyText { text: value.text },
        terms: Vec::new(),
        aliases: Vec::new(),
        score: 0,
        provider_signal: 0,
        match_reason: "calculator".to_string(),
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url: None,
    }
}

pub(crate) fn evaluate_calculator_query(query: &str) -> Option<CalculatorValue> {
    if query.chars().count() > MAX_CALCULATOR_QUERY_CHARS {
        return None;
    }
    let trimmed = query.trim_matches([' ', '\t']);
    let (expression, forced) = match trimmed.strip_prefix('=') {
        Some(rest) => (rest, true),
        None => (trimmed, false),
    };
    if !forced && is_ambiguous_text(expression) {
        return None;
    }
    let tokens = tokenize(expression)?;
    let mut parser = Parser {
        tokens: &tokens,
        position: 0,
        depth: 0,
        operators: 0,
    };
    let value = parser.bit_or()?;
    if parser.position != tokens.len() || (!forced && parser.operators == 0) {
        return None;
    }
    Some(format_value(value))
}

/// Dates, phone numbers and versions typed without spaces are not calculations.
fn is_ambiguous_text(text: &str) -> bool {
    let only = |operator: char| {
        text.contains(operator) && text.chars().all(|ch| ch.is_ascii_digit() || ch == operator)
    };
    if only('-') {
        let parts = text.split('-').map(str::len).collect::<Vec<_>>();
        return match parts.as_slice() {
            [3, 4] | [4, 1 | 2] => true,
            [_, _] => false,
            _ => true,
        };
    }
    only('/') && text.matches('/').count() >= 2
}

/// Integers stay exact until an operation has to round; a rounded value never becomes an operand
/// of a bitwise operator or a digit-for-digit integer answer.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Number {
    Int(i128),
    Float(f64),
}

impl Number {
    fn float(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        Some(if value.fract() == 0.0 && value.abs() < MAX_FLOAT_AS_INTEGER {
            Self::Int(value as i128)
        } else {
            Self::Float(value)
        })
    }

    fn as_f64(self) -> f64 {
        match self {
            Self::Int(value) => value as f64,
            Self::Float(value) => value,
        }
    }

    fn integer(self) -> Option<i128> {
        match self {
            Self::Int(value) => Some(value),
            Self::Float(_) => None,
        }
    }

    /// Exact when both sides are integers and `exact` succeeds, else floating point.
    fn arithmetic(
        self,
        right: Self,
        exact: impl FnOnce(i128, i128) -> Option<i128>,
        rounded: impl FnOnce(f64, f64) -> f64,
    ) -> Option<Self> {
        if let (Self::Int(left), Self::Int(right)) = (self, right) {
            if let Some(value) = exact(left, right) {
                return Some(Self::Int(value));
            }
        }
        Self::float(rounded(self.as_f64(), right.as_f64()))
    }

    fn bitwise(self, right: Self, operation: impl FnOnce(i128, i128) -> i128) -> Option<Self> {
        Some(Self::Int(operation(self.integer()?, right.integer()?)))
    }

    fn shift(self, token: Token, right: Self) -> Option<Self> {
        let value = self.integer()?;
        let count = u32::try_from(right.integer()?).ok().filter(|count| *count <= 63)?;
        Some(Self::Int(if token == Token::ShiftLeft {
            value.checked_mul(1_i128 << count)?
        } else {
            value >> count
        }))
    }

    fn power(self, exponent: Self) -> Option<Self> {
        self.arithmetic(
            exponent,
            |base, exponent| base.checked_pow(u32::try_from(exponent).ok()?),
            f64::powf,
        )
    }

    fn negate(self) -> Option<Self> {
        match self {
            Self::Int(value) => value.checked_neg().map(Self::Int),
            Self::Float(value) => Some(Self::Float(-value)),
        }
    }
}

fn divide_exactly(left: i128, right: i128) -> Option<i128> {
    (left.checked_rem(right)? == 0).then(|| left.checked_div(right))?
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Number(Number),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    And,
    Or,
    Xor,
    Not,
    ShiftLeft,
    ShiftRight,
    Open,
    Close,
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    if !text.is_ascii() {
        return None;
    }
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        let token = match byte {
            b' ' | b'\t' => {
                index += 1;
                continue;
            }
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'%' => Token::Percent,
            b'^' => Token::Caret,
            b'&' => Token::And,
            b'|' => Token::Or,
            b'~' => Token::Not,
            b'(' => Token::Open,
            b')' => Token::Close,
            b'<' | b'>' => {
                index += 1;
                if bytes.get(index) != Some(&byte) {
                    return None;
                }
                if byte == b'<' {
                    Token::ShiftLeft
                } else {
                    Token::ShiftRight
                }
            }
            b'0'..=b'9' | b'.' => {
                let (value, next) = lex_number(text, index)?;
                tokens.push(Token::Number(value));
                index = next;
                continue;
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                let end = bytes[index..]
                    .iter()
                    .position(|next| !next.is_ascii_alphanumeric())
                    .map_or(bytes.len(), |offset| index + offset);
                if !text[index..end].eq_ignore_ascii_case("xor") {
                    return None;
                }
                tokens.push(Token::Xor);
                index = end;
                continue;
            }
            _ => return None,
        };
        tokens.push(token);
        index += 1;
    }
    Some(tokens)
}

fn lex_number(text: &str, start: usize) -> Option<(Number, usize)> {
    let bytes = text.as_bytes();
    let digits_end = |from: usize| {
        bytes[from..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map_or(bytes.len(), |offset| from + offset)
    };
    let radix = match (bytes[start], bytes.get(start + 1)) {
        (b'0', Some(b'x' | b'X')) => Some(16),
        (b'0', Some(b'o' | b'O')) => Some(8),
        (b'0', Some(b'b' | b'B')) => Some(2),
        _ => None,
    };
    let (value, end) = if let Some(radix) = radix {
        let from = start + 2;
        let end = bytes[from..]
            .iter()
            .position(|byte| !byte.is_ascii_alphanumeric())
            .map_or(bytes.len(), |offset| from + offset);
        (Number::Int(i128::from_str_radix(&text[from..end], radix).ok()?), end)
    } else {
        let mut end = digits_end(start);
        if bytes.get(end) == Some(&b'.') {
            let fraction = end + 1;
            end = digits_end(fraction);
            if end == fraction {
                return None;
            }
        }
        if matches!(bytes.get(end), Some(b'e' | b'E')) {
            let mut exponent = end + 1;
            if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
                exponent += 1;
            }
            end = digits_end(exponent);
            if end == exponent {
                return None;
            }
        }
        let literal = &text[start..end];
        let value = match literal.parse::<i128>() {
            Ok(value) => Number::Int(value),
            Err(_) => float_literal(literal)?,
        };
        (value, end)
    };
    let follows_number = bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'));
    (!follows_number).then_some((value, end))
}

/// `1e15` is an exact integer even though it is written as a float.
fn float_literal(literal: &str) -> Option<Number> {
    let value = literal.parse::<f64>().ok()?;
    let integral = value.fract() == 0.0 && value.abs() < MAX_EXACT_INTEGER as f64;
    // With a fraction part the text may have been rounded to reach an integer.
    if integral && !literal.contains('.') {
        return Some(Number::Int(value as i128));
    }
    Number::float(value)
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    depth: usize,
    operators: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.position).copied()
    }

    fn take(&mut self, accepted: &[Token]) -> Option<Token> {
        let token = self.peek().filter(|token| accepted.contains(token))?;
        self.position += 1;
        Some(token)
    }

    fn operator(&mut self, accepted: &[Token]) -> Option<Token> {
        let token = self.take(accepted)?;
        self.operators += 1;
        Some(token)
    }

    fn nested(&mut self, parse: impl FnOnce(&mut Self) -> Option<Number>) -> Option<Number> {
        if self.depth >= MAX_CALCULATOR_NESTING_DEPTH {
            return None;
        }
        self.depth += 1;
        let value = parse(self);
        self.depth -= 1;
        value
    }

    fn bit_or(&mut self) -> Option<Number> {
        let mut left = self.bit_xor()?;
        while self.operator(&[Token::Or]).is_some() {
            left = left.bitwise(self.bit_xor()?, |left, right| left | right)?;
        }
        Some(left)
    }

    fn bit_xor(&mut self) -> Option<Number> {
        let mut left = self.bit_and()?;
        while self.operator(&[Token::Xor]).is_some() {
            left = left.bitwise(self.bit_and()?, |left, right| left ^ right)?;
        }
        Some(left)
    }

    fn bit_and(&mut self) -> Option<Number> {
        let mut left = self.shift()?;
        while self.operator(&[Token::And]).is_some() {
            left = left.bitwise(self.shift()?, |left, right| left & right)?;
        }
        Some(left)
    }

    fn shift(&mut self) -> Option<Number> {
        let mut left = self.sum()?;
        while let Some(token) = self.operator(&[Token::ShiftLeft, Token::ShiftRight]) {
            left = left.shift(token, self.sum()?)?;
        }
        Some(left)
    }

    fn sum(&mut self) -> Option<Number> {
        let mut left = self.product()?;
        while let Some(token) = self.operator(&[Token::Plus, Token::Minus]) {
            let right = self.product()?;
            left = if token == Token::Plus {
                left.arithmetic(right, i128::checked_add, |left, right| left + right)?
            } else {
                left.arithmetic(right, i128::checked_sub, |left, right| left - right)?
            };
        }
        Some(left)
    }

    fn product(&mut self) -> Option<Number> {
        let mut left = self.unary()?;
        while let Some(token) = self.operator(&[Token::Star, Token::Slash, Token::Percent]) {
            let right = self.unary()?;
            left = match token {
                Token::Star => {
                    left.arithmetic(right, i128::checked_mul, |left, right| left * right)?
                }
                Token::Slash => left.arithmetic(right, divide_exactly, |left, right| left / right)?,
                _ => left.arithmetic(right, i128::checked_rem, |left, right| left % right)?,
            };
        }
        Some(left)
    }

    fn unary(&mut self) -> Option<Number> {
        if self.take(&[Token::Minus]).is_some() {
            return self.nested(Self::unary)?.negate();
        }
        if self.operator(&[Token::Not]).is_some() {
            return Some(Number::Int(!self.nested(Self::unary)?.integer()?));
        }
        let base = self.primary()?;
        if self.operator(&[Token::Caret]).is_some() {
            // The exponent is itself a unary, which makes `^` right associative.
            return base.power(self.unary()?);
        }
        Some(base)
    }

    fn primary(&mut self) -> Option<Number> {
        match self.peek()? {
            Token::Number(value) => {
                self.position += 1;
                Some(value)
            }
            Token::Open => {
                self.position += 1;
                let value = self.nested(Self::bit_or)?;
                self.take(&[Token::Close])?;
                Some(value)
            }
            _ => None,
        }
    }
}

fn format_value(value: Number) -> CalculatorValue {
    // Beyond 64 bits the scientific text is visibly approximate rather than silently rounded.
    let exact = value
        .integer()
        .and_then(|value| i64::try_from(value).ok());
    let radix = exact.filter(|value| (0..=MAX_EXACT_INTEGER).contains(&i128::from(*value)));
    CalculatorValue {
        text: exact.map_or_else(|| format_decimal(value.as_f64()), |value| value.to_string()),
        hex: radix.map(|value| format!("0x{value:X}")),
        bin: radix.map(|value| format!("0b{value:b}")),
    }
}

fn format_decimal(value: f64) -> String {
    let scientific = format!("{:.*e}", SIGNIFICANT_DIGITS - 1, value.abs());
    let sign = if value < 0.0 { "-" } else { "" };
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent = exponent.parse::<i32>().unwrap_or(0);
    let digits = mantissa.replace('.', "");
    let digits = digits.trim_end_matches('0');
    if !(-7..SIGNIFICANT_DIGITS as i32).contains(&exponent) {
        let (lead, rest) = digits.split_at(1.min(digits.len()));
        let point = if rest.is_empty() { "" } else { "." };
        return format!("{sign}{lead}{point}{rest}e{exponent}");
    }
    if exponent < 0 {
        let zeros = "0".repeat((-exponent - 1) as usize);
        return format!("{sign}0.{zeros}{digits}");
    }
    let whole_len = exponent as usize + 1;
    if digits.len() <= whole_len {
        return format!("{sign}{digits:0<whole_len$}");
    }
    let (whole, fraction) = digits.split_at(whole_len);
    format!("{sign}{whole}.{fraction}")
}
