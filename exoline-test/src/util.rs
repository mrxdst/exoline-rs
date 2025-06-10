use std::{error::Error, fmt::Display, future::IntoFuture, time::Duration};

use exoline::client::Variant;
use tokio::select;

#[derive(Debug)]
pub enum AbortReason {
    Timeout,
    Cancel,
}

impl Display for AbortReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AbortReason::Timeout => write!(f, "Timeout"),
            AbortReason::Cancel => write!(f, "Cancel"),
        }
    }
}

impl Error for AbortReason {}

pub async fn timeout_or_cancel<F>(timeout: Duration, future: F) -> Result<F::Output, AbortReason>
where
    F: IntoFuture,
{
    select! {
        result = future => Ok(result),
        _ = tokio::time::sleep(timeout) => Err(AbortReason::Timeout),
        _ = tokio::signal::ctrl_c() => Err(AbortReason::Cancel)
    }
}

pub trait PrettyDisplay {
    fn pretty(&self) -> String;
}

impl PrettyDisplay for f32 {
    fn pretty(&self) -> String {
        let abs = f32::abs(*self);
        if abs != 0.0 && (abs >= 1e16 || abs <= 1e-6) {
            format!("{self:e}")
        } else {
            format!("{self}")
        }
    }
}

pub fn format_variant(value: &Variant) -> String {
    match value {
        Variant::Huge(value) => format!("{value}"),
        Variant::Index(value) => format!("{value}"),
        Variant::Integer(value) => format!("{value}"),
        Variant::Logic(value) => format!("{value}"),
        Variant::Real(value) => value.pretty(),
        Variant::String(value) => format!("{value}"),
    }
}
