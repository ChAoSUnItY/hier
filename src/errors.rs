use std::sync::PoisonError;

use jni::errors::{JniError, StartJvmError};
use thiserror::Error;

pub type HierResult<T> = Result<T, HierError>;

#[derive(Error, Debug)]
pub enum HierError {
    #[error(transparent)]
    JavaError(#[from] jni::errors::Error),
    #[error(transparent)]
    JniError(#[from] JniError),
    #[error(transparent)]
    StartJvmError(#[from] StartJvmError),
    #[error("unable to access to class cache, reason: {0}")]
    CacheAccessError(&'static str),
}

impl<T> From<PoisonError<T>> for HierError {
    fn from(_value: PoisonError<T>) -> Self {
        Self::CacheAccessError("PoisonError")
    }
}