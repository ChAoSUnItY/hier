use std::sync::Arc;

use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use once_cell::sync::OnceCell;

use crate::errors::HierResult as Result;

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
