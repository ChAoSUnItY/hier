use jni::objects::{JClass, JValueOwned, JObjectArray};
use jni::JNIEnv;

pub trait HierarchyClass<'local> {
    fn is_interface(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<bool>;

    fn is_assignable_from(
        &self,
        env: &mut JNIEnv<'local>,
        other: &JClass<'local>,
    ) -> jni::errors::Result<bool>;

    fn name(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<String>;

    fn super_class(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<Option<JClass<'local>>>;

    fn interfaces(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<Vec<JClass<'local>>>;
}

impl<'local> HierarchyClass<'local> for JClass<'local> {
    fn is_interface(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<bool> {
        env.call_method(self, "isInterface", "()Z", &[])
            .and_then(JValueOwned::z)
    }

    fn is_assignable_from(
        &self,
        env: &mut JNIEnv<'local>,
        other: &JClass<'local>,
    ) -> jni::errors::Result<bool> {
        env.is_assignable_from(self, other)
    }

    fn name(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<String> {
        let class_name = env
            .call_method(self, "getName", "()Ljava/lang/String;", &[])
            .and_then(JValueOwned::l)?;
        unsafe {
            env.get_string_unchecked((&class_name).into())
                .map(|java_str| java_str.into())
        }
        .map(|name: String| name.replace(".", "/"))
    }

    fn super_class(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<Option<JClass<'local>>> {
        env.get_superclass(self)
    }

    fn interfaces(&self, env: &mut JNIEnv<'local>) -> jni::errors::Result<Vec<JClass<'local>>> {
        let interfaces_arr: JObjectArray<'local> = env
            .call_method(self, "getInterfaces", "()[Ljava/lang/Class;", &[])
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
