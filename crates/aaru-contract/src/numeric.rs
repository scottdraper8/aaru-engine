use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

pub const SCALE: i128 = 1_000_000;
pub const MAX_DECIMAL_BYTES: usize = 128;
pub const MAX_EXPONENT: i32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericError {
    Syntax,
    Overflow,
    Inexact,
    DivisionByZero,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed(i64);

impl Fixed {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1_000_000);
    pub const fn from_scaled(value: i64) -> Self {
        Self(value)
    }
    pub const fn scaled(self) -> i64 {
        self.0
    }
    pub fn checked_add(self, other: Self) -> Result<Self, NumericError> {
        narrow(
            i128::from(self.0)
                .checked_add(i128::from(other.0))
                .ok_or(NumericError::Overflow)?,
        )
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, NumericError> {
        narrow(
            i128::from(self.0)
                .checked_sub(i128::from(other.0))
                .ok_or(NumericError::Overflow)?,
        )
    }
    pub fn checked_mul(self, other: Self) -> Result<Self, NumericError> {
        narrow(round_ratio(
            i128::from(self.0)
                .checked_mul(i128::from(other.0))
                .ok_or(NumericError::Overflow)?,
            SCALE,
        )?)
    }
    pub fn checked_div(self, other: Self) -> Result<Self, NumericError> {
        narrow(round_ratio(
            i128::from(self.0)
                .checked_mul(SCALE)
                .ok_or(NumericError::Overflow)?,
            i128::from(other.0),
        )?)
    }
    pub fn from_decimal(lexeme: &str) -> Result<Self, NumericError> {
        decimal(lexeme, 6)
    }
    pub fn from_percentage_points(lexeme: &str) -> Result<Self, NumericError> {
        decimal(lexeme, 4)
    }
}

fn narrow(value: i128) -> Result<Fixed, NumericError> {
    i64::try_from(value)
        .map(Fixed)
        .map_err(|_| NumericError::Overflow)
}

pub fn round_ratio(numerator: i128, denominator: i128) -> Result<i128, NumericError> {
    if denominator == 0 {
        return Err(NumericError::DivisionByZero);
    }
    let q = numerator
        .checked_div(denominator)
        .ok_or(NumericError::Overflow)?;
    let r = numerator
        .checked_rem(denominator)
        .ok_or(NumericError::Overflow)?;
    let remainder = r.unsigned_abs();
    let divisor = denominator.unsigned_abs();
    if remainder > divisor - remainder || (remainder == divisor - remainder && q % 2 != 0) {
        q.checked_add(if (numerator < 0) == (denominator < 0) {
            1
        } else {
            -1
        })
        .ok_or(NumericError::Overflow)
    } else {
        Ok(q)
    }
}

fn decimal(text: &str, precision: i32) -> Result<Fixed, NumericError> {
    if text.is_empty() || text.len() > MAX_DECIMAL_BYTES {
        return Err(NumericError::Syntax);
    }
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((m, e)) => {
            let digits = e.strip_prefix(['+', '-']).unwrap_or(e);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(NumericError::Syntax);
            }
            let exponent = e.parse::<i32>().map_err(|_| NumericError::Overflow)?;
            if exponent.unsigned_abs() > MAX_EXPONENT as u32 {
                return Err(NumericError::Overflow);
            }
            (m, exponent)
        }
        None => (text, 0),
    };
    let (negative, unsigned) = match mantissa.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, mantissa),
    };
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || (whole.len() > 1 && whole.starts_with('0'))
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || (unsigned.contains('.') && fraction.is_empty())
    {
        return Err(NumericError::Syntax);
    }
    let digits = format!("{whole}{fraction}");
    let mut value = digits.parse::<i128>().map_err(|_| NumericError::Overflow)?;
    let power = precision
        .checked_add(exponent)
        .and_then(|p| p.checked_sub(i32::try_from(fraction.len()).ok()?))
        .ok_or(NumericError::Overflow)?;
    if value == 0 {
        return Ok(Fixed::ZERO);
    }
    let factor = 10_i128
        .checked_pow(power.unsigned_abs())
        .ok_or(NumericError::Overflow)?;
    if power >= 0 {
        value = value.checked_mul(factor).ok_or(NumericError::Overflow)?;
    } else {
        if value % factor != 0 {
            return Err(NumericError::Inexact);
        }
        value /= factor;
    }
    if negative {
        value = value.checked_neg().ok_or(NumericError::Overflow)?;
    }
    narrow(value)
}

impl Serialize for Fixed {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Fixed {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let value = text.parse::<i64>().map_err(D::Error::custom)?;
        if value.to_string() != text {
            return Err(D::Error::custom("expected canonical scaled integer string"));
        }
        Ok(Self(value))
    }
}
