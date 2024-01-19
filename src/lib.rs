use std::sync::{Arc, Mutex, Once};

use jni::{AttachGuard, InitArgsBuilder, JavaVM, JNIVersion};
use jni::objects::{JClass, JObjectArray, JValueOwned};

#[cfg(feature = "graph")]
mod graph;

pub extern crate jni;

pub trait HierarchyClass<'local> {
    fn is_interface(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<bool>;

    fn is_assignable_from(&self, env: &mut AttachGuard<'local>, other: &JClass<'local>) -> jni::errors::Result<bool>;

    fn name(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<String>;

    fn super_class(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<Option<JClass<'local>>>;

    fn interfaces(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<Vec<JClass<'local>>>;
}

impl<'local> HierarchyClass<'local> for JClass<'local> {
    fn is_interface(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<bool> {
        env
            .call_method(self, "isInterface", "()Z", &[])
            .and_then(JValueOwned::z)
    }

    fn is_assignable_from(&self, env: &mut AttachGuard<'local>, other: &JClass<'local>) -> jni::errors::Result<bool> {
        env.is_assignable_from(self, other)
    }

    fn name(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<String> {
        let class_name = env
            .call_method(self, "getName", "()Ljava/lang/String;", &[])
            .and_then(JValueOwned::l)?;
        unsafe { env.get_string_unchecked((&class_name).into()).map(|java_str| java_str.into()) }.map(|name: String| name.replace(".", "/"))
    }

    fn super_class(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<Option<JClass<'local>>> {
        env.get_superclass(self)
    }

    fn interfaces(&self, env: &mut AttachGuard<'local>) -> jni::errors::Result<Vec<JClass<'local>>> {
        let interfaces_arr: JObjectArray<'local> = env.call_method(self, "getInterfaces", "()[Ljava/lang/Class;", &[])
            .and_then(JValueOwned::l)?
            .into();
        let interfaces_len = env.get_array_length(&interfaces_arr)?;
        let mut interfaces = Vec::with_capacity(interfaces_len as usize);

        for i in 0..interfaces_len {
            interfaces.push(env.get_object_array_element(&interfaces_arr, i)?.into());
        }

        Ok(interfaces)
    }
}

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

fn env() -> &'static Arc<Mutex<AttachGuard<'static>>> {
    static mut ENV: Option<Arc<Mutex<AttachGuard>>> = None;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let env = jvm().attach_current_thread().unwrap();

        unsafe {
            ENV = Some(Arc::new(Mutex::new(env)));
        }
    });

    unsafe { ENV.as_ref().unwrap() }
}

pub fn jclass<'local>(env: &mut AttachGuard<'local>, class_name: &str) -> jni::errors::Result<JClass<'local>> {
    env.find_class(class_name)
}

/// Get common super class' simple name.
pub fn common_super_class<'local>(env: &mut AttachGuard<'local>, class1: JClass<'local>, class2: JClass<'local>) -> jni::errors::Result<String> {
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
            None => return Ok(String::from(OBJECT_CLASS_PATH))
        };

        !cls2.is_assignable_from(env, &cls1)?
    } {}

    cls1.name(env)
}

#[cfg(test)]
mod test {
    use crate::{common_super_class, env, HierarchyClass, jclass};

    #[test]
    fn test_number_common() -> jni::errors::Result<()> {
        let mut env = env().lock().unwrap();
        let class1 = jclass(&mut env, "java/lang/Integer")?;
        let class2 = jclass(&mut env, "java/lang/Float")?;
        assert_eq!("java/lang/Number", common_super_class(&mut env, class1, class2)?);

        Ok(())
    }

    #[test]
    fn test_interfaces() -> jni::errors::Result<()> {
        let mut env = env().lock().unwrap();
        let class = jclass(&mut env, "java/lang/Integer")?;
        let interfaces = class.interfaces(&mut env)?;
        let interface_names = interfaces.iter().map(|interface| interface.name(&mut env)).collect::<jni::errors::Result<Vec<_>>>()?;

        assert_eq!(vec!["java/lang/Comparable", "java/lang/constant/Constable", "java/lang/constant/ConstantDesc"], interface_names);

        Ok(())
    }
}
