#![doc = include_str!("../README.md")]

use std::sync::Arc;

use errors::HierResult as Result;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use once_cell::sync::OnceCell;

pub mod class;
mod errors;
#[cfg(feature = "graph")]
pub mod graph;
pub mod version;

pub extern crate jni;

/// Get JVM instance, initialize if does not exist.
fn jvm() -> Result<&'static Arc<JavaVM>> {
    static JVM: OnceCell<Arc<JavaVM>> = OnceCell::new();

    JVM.get_or_try_init(|| -> Result<Arc<JavaVM>> {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()?;

        let jvm = JavaVM::new(jvm_args)?;

        Ok(Arc::new(jvm))
    })
}

/// Get JNI environment instance, notice that the thread is attached permanently.
pub fn jni_env() -> Result<JNIEnv<'static>> {
    jvm().and_then(|jvm| jvm.attach_current_thread_permanently().map_err(Into::into))
}
