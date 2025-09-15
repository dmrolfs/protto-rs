use crate::analysis::expect_analysis::ExpectMode;
use crate::analysis::field_analysis::FieldProcessingContext;
use crate::field::info::RustFieldInfo;

pub mod mode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorMode {
    None,
    Panic,
    Error,
    Default(Option<String>),
}

impl ErrorMode {
    pub fn from_field_context(ctx: &FieldProcessingContext, rust: &RustFieldInfo) -> Self {
        match rust.expect_mode {
            ExpectMode::Panic => Self::Panic,
            ExpectMode::Error => Self::Error,
            ExpectMode::None => {
                if rust.has_default || ctx.default_fn.is_some() {
                    Self::Default(ctx.default_fn.clone())
                } else {
                    Self::None
                }
            }
        }
    }
}