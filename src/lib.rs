use std::sync::{Arc, Once};

use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};

pub mod class;
#[cfg(feature = "graph")]
pub mod graph;
pub mod version;

pub extern crate jni;

/// Get JVM instance, initialize if does not exist.
fn jvm() -> &'static Arc<JavaVM> {
    static mut JVM: Option<Arc<JavaVM>> = None;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()
            .unwrap_or_else(|e| panic!("{:#?}", e));

        let jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| panic!("{:#?}", e));

        unsafe {
            JVM = Some(Arc::new(jvm));
        }
    });

    unsafe { JVM.as_ref().unwrap() }
}

pub fn jni_env() -> JNIEnv<'static> {
    jvm().attach_current_thread_permanently().unwrap()
}
