use std::sync::PoisonError;

use jni::errors::JniError;
use thiserror::Error;

pub type HierResult<T> = Result<T, HierError>;

#[derive(Error, Debug)]
pub enum HierError {
    #[error(transparent)]
    JavaError(#[from] jni::errors::Error),
    #[error(transparent)]
    #[cfg(feature = "invocation")]
    JvmError(#[from] jni::JvmError),
    #[error(transparent)]
    #[cfg(feature = "invocation")]
    StartJvmError(#[from] jni::errors::StartJvmError),
    #[error(transparent)]
    JniError(#[from] JniError),
    #[error("unable to access to class cache, reason: {0}")]
    CacheAccessError(&'static str),
    #[error("unable to find the class {0} in the cache, Class probably had been freed up")]
    DanglingClassError(String),
}

impl<T> From<PoisonError<T>> for HierError {
    fn from(_value: PoisonError<T>) -> Self {
        Self::CacheAccessError("PoisonError")
    }
}
