use std::sync::{Arc, Once};
use jni::JavaVM;
use jni::{InitArgsBuilder, JNIVersion};

pub const OBJECT_CLASS_PATH: &'static str = "java/lang/Object";

pub fn jvm() -> &'static Arc<JavaVM> {
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

fn get_common_super_class(class1: &str, class2: &str) -> String {
    let mut env = jvm().attach_current_thread().unwrap();
    let mut cls1 = env.find_class(class1).unwrap();
    let cls2 = env.find_class(class2).unwrap();

    if env.is_assignable_from(&cls2, &cls1).unwrap() {
        return class1.to_string();
    }

    if env.is_assignable_from(&cls1, &cls2).unwrap() {
        return class2.to_string();
    }

    if env
        .call_method(&cls1, "isInterface", "()Z", &[])
        .unwrap()
        .z()
        .unwrap()
        || env
            .call_method(&cls2, "isInterface", "()Z", &[])
            .unwrap()
            .z()
            .unwrap()
    {
        return String::from("java/lang/Object");
    }

    while {
        cls1 = match env.get_superclass(&cls1).unwrap() {
            Some(cls) => cls,
            None => return String::from(OBJECT_CLASS_PATH)
        };

        !env.is_assignable_from(&cls2, &cls2).unwrap()
    } {}

    let cls_name = env
        .call_method(&cls1, "getName", "()Ljava/lang/String;", &[])
        .unwrap()
        .l()
        .unwrap();
    let cls_name: String = unsafe { env.get_string_unchecked((&cls_name).into()) }
        .unwrap()
        .into();

    cls_name.replace(".", "/")
}

fn main() {
    println!(
        "{}",
        get_common_super_class("java/lang/Integer", "java/lang/Float")
    );
}
