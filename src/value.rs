use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Uninitialized,  // awk variables start as "" / 0 depending on context
}

impl Value {
    pub fn to_f64(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::String(s) => s.trim().parse().unwrap_or(0.0),
            Value::Uninitialized => 0.0,
        }
    }

    pub fn to_string_val(&self) -> String {
        match self {
            Value::Number(n) => {
                // awk prints integers without decimal point
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::String(s) => s.clone(),
            Value::Uninitialized => String::new(),
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Uninitialized => false,
        }
    }

    pub fn is_numeric_string(s: &str) -> bool {
        s.trim().parse::<f64>().is_ok()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_val())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            // mixed: if both look numeric, compare numerically
            (Value::String(s), Value::Number(n)) |
            (Value::Number(n), Value::String(s)) => {
                if let Ok(parsed) = s.trim().parse::<f64>() {
                    parsed == *n
                } else {
                    false
                }
            }
            (Value::Uninitialized, Value::Uninitialized) => true,
            (Value::Uninitialized, Value::Number(n)) => *n == 0.0,
            (Value::Uninitialized, Value::String(s)) => s.is_empty(),
            (Value::Number(n), Value::Uninitialized) => *n == 0.0,
            (Value::String(s), Value::Uninitialized) => s.is_empty(),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
            _ => self.to_string_val().partial_cmp(&other.to_string_val()),
        }
    }
}