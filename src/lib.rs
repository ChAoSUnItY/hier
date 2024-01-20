use std::sync::{Arc, Once};

use class::HierarchyClass;
use jni::objects::{JClass, JValueGen};
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jvm::JvmVersion;

mod class;
#[cfg(feature = "graph")]
mod graph;
mod jvm;

pub extern crate jni;

pub const OBJECT_CLASS_PATH: &'static str = "java/lang/Object";

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

pub fn get_jvm_version(env: &mut JNIEnv<'static>) -> jni::errors::Result<JvmVersion> {
    let sys_class = env.find_class("java/lang/System")?;
    let property = env.new_string("java.specification.version")?;
    let version = env
        .call_static_method(
            sys_class,
            "getProperty",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[(&property).into()],
        )
        .and_then(JValueGen::l)?;

    env.get_string((&version).into())
        .map(|java_str| JvmVersion::from(Into::<String>::into(java_str)))
}

pub fn jclass<'local>(
    env: &mut JNIEnv<'local>,
    class_name: &str,
) -> jni::errors::Result<JClass<'local>> {
    env.find_class(class_name)
}

/// Get common super class' simple name.
pub fn common_super_class<'local>(
    env: &mut JNIEnv<'local>,
    class1: JClass<'local>,
    class2: JClass<'local>,
) -> jni::errors::Result<String> {
    let mut cls1: JClass = class1;
    let cls2: JClass = class2;

    if cls2.is_assignable_from(env, &cls1)? {
        return cls1.name(env);
    }

    if cls1.is_assignable_from(env, &cls2)? {
        return cls2.name(env);
    }

    if cls1.is_interface(env)? || cls2.is_interface(env)? {
        return Ok(String::from(OBJECT_CLASS_PATH));
    }

    while {
        cls1 = match cls1.super_class(env)? {
            Some(cls) => cls,
            None => return Ok(String::from(OBJECT_CLASS_PATH)),
        };

        !cls2.is_assignable_from(env, &cls1)?
    } {}

    cls1.name(env)
}

#[cfg(test)]
mod test {
    use crate::{
        class::HierarchyClass, common_super_class, get_jvm_version, jclass, jni_env,
        jvm::JvmVersion,
    };

    #[test]
    fn test_number_common() -> jni::errors::Result<()> {
        let mut env = jni_env();
        let class1 = jclass(&mut env, "java/lang/Integer")?;
        let class2 = jclass(&mut env, "java/lang/Float")?;
        assert_eq!(
            "java/lang/Number",
            common_super_class(&mut env, class1, class2)?
        );

        Ok(())
    }

    #[test]
    #[cfg_attr(
        not(any(
            jvm_v0, jvm_v1, jvm_v2, jvm_v3, jvm_v4, jvm_v5, jvm_v6, jvm_v7, jvm_v8, jvm_v9,
            jvm_v10, jvm_v11, jvm_v12, jvm_v13, jvm_v14, jvm_v15, jvm_v16, jvm_v17, jvm_v18,
            jvm_v19, jvm_v20, jvm_v21, jvm_v22, jvm_v23,
        )),
        ignore
    )]
    /// Tests all possible jvm versions
    fn test_jvm_version() -> jni::errors::Result<()> {
        let current_jvm_version: JvmVersion = if cfg!(jvm_v0) {
            JvmVersion::V0
        } else if cfg!(jvm_v1) {
            JvmVersion::V1
        } else if cfg!(jvm_v2) {
            JvmVersion::V2
        } else if cfg!(jvm_v3) {
            JvmVersion::V3
        } else if cfg!(jvm_v4) {
            JvmVersion::V4
        } else if cfg!(jvm_v5) {
            JvmVersion::V5
        } else if cfg!(jvm_v6) {
            JvmVersion::V6
        } else if cfg!(jvm_v7) {
            JvmVersion::V7
        } else if cfg!(jvm_v8) {
            JvmVersion::V8
        } else if cfg!(jvm_v9) {
            JvmVersion::V9
        } else if cfg!(jvm_v10) {
            JvmVersion::V10
        } else if cfg!(jvm_v11) {
            JvmVersion::V11
        } else if cfg!(jvm_v12) {
            JvmVersion::V12
        } else if cfg!(jvm_v13) {
            JvmVersion::V13
        } else if cfg!(jvm_v14) {
            JvmVersion::V14
        } else if cfg!(jvm_v15) {
            JvmVersion::V15
        } else if cfg!(jvm_v16) {
            JvmVersion::V16
        } else if cfg!(jvm_v17) {
            JvmVersion::V17
        } else if cfg!(jvm_v18) {
            JvmVersion::V18
        } else if cfg!(jvm_v19) {
            JvmVersion::V19
        } else if cfg!(jvm_v20) {
            JvmVersion::V20
        } else if cfg!(jvm_v21) {
            JvmVersion::V21
        } else if cfg!(jvm_v22) {
            JvmVersion::V22
        } else if cfg!(jvm_v23) {
            JvmVersion::V23
        } else {
            panic!("Unsupported JVM version");
        };

        let mut env = jni_env();
        let version = get_jvm_version(&mut env)?;

        assert_eq!(current_jvm_version, version);

        Ok(())
    }

    #[test]
    #[cfg_attr(not(any(jvm_v8, jvm_v11, jvm_v17, jvm_v21)), attr)]
    /// Tests all implemented interfaces on `java/lang/Integer`
    /// (non recursively to super class which is `java/lang/Number`)
    fn test_interfaces() -> jni::errors::Result<()> {
        let implemented_interfaces = if cfg!(any(jvm_v8, jvm_v11)) {
            vec!["java/lang/Comparable"]
        } else if cfg!(any(jvm_v17, jvm_v21)) {
            vec![
                "java/lang/Comparable",
                "java/lang/constant/Constable",
                "java/lang/constant/ConstantDesc",
            ]
        } else {
            panic!("Unsupported JVM version");
        };

        let mut env = jni_env();
        let class = jclass(&mut env, "java/lang/Integer")?;
        let interfaces = class.interfaces(&mut env)?;
        let interface_names = interfaces
            .iter()
            .map(|interface| interface.name(&mut env))
            .collect::<jni::errors::Result<Vec<_>>>()?;

        assert_eq!(implemented_interfaces, interface_names);

        Ok(())
    }
}
