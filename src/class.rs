use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use jni::descriptors::Desc;
use jni::objects::{GlobalRef, JClass, JObjectArray, JValueGen, JValueOwned};
use jni::JNIEnv;

use crate::errors::HierResult as Result;
use crate::version::JavaVersion;

pub const OBJECT_CLASS_PATH: &'static str = "java/lang/Object";

pub type ClassCache = HashMap<String, GlobalRef>;

fn class_cache() -> &'static RwLock<ClassCache> {
    static CLASS_CACHE: OnceLock<RwLock<ClassCache>> = OnceLock::new();
    CLASS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

// fn jclass_cache() -> &'static mut ClassCache {
//     static CACHE: OnceLock<ClassCache> = OnceLock::new();
//     CACHE.get_or_init(|| HashMap::new())
// }

/// Fetch an [GlobalRef] (JClass) either from cache if already fetched before, or directly
/// from JNI interface if not. After each successful fetching operation, [GlobalRef] (JClass)
/// instance will exist until the termination of program, if this is not desired,
/// use [free_jclass_cache] to free cache.
fn jclass(env: &mut JNIEnv, class_path: &str) -> Result<GlobalRef> {
    let cache = class_cache().read()?;

    if let Some(cached_class) = cache.get(class_path) {
        Ok(cached_class.clone())
    } else {
        drop(cache);
        let mut cache = class_cache().write()?;

        let class = env.find_class(class_path)?;
        let glob_ref = env.new_global_ref(class)?;

        Ok(cache
            .entry(class_path.to_string())
            .or_insert(glob_ref)
            .clone())
    }
}

/// Fetch an [GlobalRef] (JClass) from cache, either:
/// 1. Existing JClass with same class path are same instance, then return cached one.
/// 2. Existing JClass with same class path are not same instance, then cache and return the
///    provided one
/// 3. JClass is not cached, then cache and return the provided one
fn jclass_from_instance<'local, 'other_local, T>(
    env: &mut JNIEnv<'local>,
    instance: T,
) -> Result<GlobalRef> where T: Desc<'local, JClass<'other_local>> {
    let cache = class_cache().read()?;

    let instance = instance.lookup(env)?;
    let instance = instance.as_ref();
    let instance_class_path = env.class_name(instance)?;

    if let Some(glob_ref) = cache.get(&instance_class_path) {
        if env.is_same_object(&glob_ref, instance)? {
            return Ok(glob_ref.clone());
        }
    }

    drop(cache);

    let mut cache = class_cache().write()?;
    let instance_glob_ref = env.new_global_ref(instance)?;

    cache.insert(instance_class_path.to_string(), instance_glob_ref.clone());

    Ok(instance_glob_ref)
}

/// Frees jclass cache.
fn free_jclass_cache() -> Result<()> {
    class_cache().write()?.clear();

    Ok(())
}

/// The additional definition for [JNIEnv], used for define
/// [JClass] caching (see [HierExt::lookup_class] and [HierExt::free_lookup])
/// and other useful class-related functions.
pub trait HierExt<'local> {
    /// Gets the java version currently the jni environment is running on.
    fn get_java_version(&mut self) -> Result<JavaVersion>;

    /// Lookups class from given class path, if class is found, then caches and returns
    /// it.
    fn lookup_class(&mut self, class_path: &str) -> Result<GlobalRef>;

    /// Lookups superclass from given class instance, if superclass is found, then
    /// caches and returns, otherwise, returns [None].
    fn lookup_superclass<'other_local, T>(&mut self, class: T) -> Result<Option<GlobalRef>>
    where
        T: Desc<'local, JClass<'other_local>>;

    /// Frees the class cache.
    fn free_lookup(&mut self) -> Result<()> {
        free_jclass_cache()
    }

    /// Determines if the given class is an interface type.
    fn is_interface<'other_local, T>(&mut self, class: T) -> Result<bool>
    where
        T: Desc<'local, JClass<'other_local>>;

    /// Returns the given class' class path.
    fn class_name<'other_local, T>(&mut self, class: T) -> Result<String>
    where
        T: Desc<'local, JClass<'other_local>>;

    /// Returns the given class' derived interfaces.
    ///
    /// # Example
    ///
    /// Assuming `java/lang/Integer` has the following declaration:
    /// ```java
    /// public class Integer extends Number implements Comparable<Integer> {
    ///     // ...
    /// }
    /// ```
    ///
    /// Calling [HierExt::interfaces] returns `[JClass("java/lang/Comparable")]`,
    /// notice that this does collects interfaces derived by superclasses.
    fn interfaces<'other_local, T>(&mut self, class: T) -> Result<Vec<GlobalRef>>
    where
        T: Desc<'local, JClass<'other_local>>;

    /// Returns the given 2 class' most common superclass.
    ///
    /// If 1 of the classes is interface, then returns `JClass("java/lang/Object")`.
    fn common_superclass<'other_local_1: 'local, 'other_local_2: 'local, T, U>(
        &mut self,
        class1: T,
        class2: U,
    ) -> Result<GlobalRef>
    where
        T: Desc<'local, JClass<'other_local_1>>,
        U: Desc<'local, JClass<'other_local_2>>;
}

impl<'local> HierExt<'local> for JNIEnv<'local> {
    fn get_java_version(&mut self) -> Result<JavaVersion> {
        let sys_class = self.find_class("java/lang/System")?;
        let property = self.new_string("java.specification.version")?;
        let version = self
            .call_static_method(
                sys_class,
                "getProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[(&property).into()],
            )
            .and_then(JValueGen::l)?;

        self.get_string((&version).into())
            .map(|java_str| JavaVersion::from(Into::<String>::into(java_str)))
            .map_err(Into::into)
    }

    fn lookup_class(&mut self, class_path: &str) -> Result<GlobalRef> {
        jclass(self, class_path)
    }

    fn lookup_superclass<'other_local, T>(&mut self, class: T) -> Result<Option<GlobalRef>>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let Some(superclass_instance) = self.get_superclass(class.as_ref())? else {
            return Ok(None);
        };

        jclass_from_instance(self, &superclass_instance).map(Option::Some)
    }

    fn is_interface<'other_local, T>(&mut self, class: T) -> Result<bool>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;

        self.call_method(class.as_ref(), "isInterface", "()Z", &[])
            .and_then(JValueOwned::z)
            .map_err(Into::into)
    }

    fn class_name<'other_local, T>(&mut self, class: T) -> Result<String>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let class_name = self
            .call_method(class.as_ref(), "getName", "()Ljava/lang/String;", &[])
            .and_then(JValueOwned::l)?;

        self.get_string((&class_name).into())
            .map(|name| Into::<String>::into(name).replace(".", "/"))
            .map_err(Into::into)
    }

    fn interfaces<'other_local, T>(&mut self, class: T) -> Result<Vec<GlobalRef>>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let interfaces_arr: JObjectArray<'local> = self
            .call_method(class.as_ref(), "getInterfaces", "()[Ljava/lang/Class;", &[])
            .and_then(JValueOwned::l)?
            .into();
        let interfaces_len = self.get_array_length(&interfaces_arr)?;
        let mut interfaces = Vec::with_capacity(interfaces_len as usize);

        for i in 0..interfaces_len {
            let class = self.get_object_array_element(&interfaces_arr, i)?;
            let class: &JClass = (&class).into();
            let cached_class = jclass_from_instance(self, class)?;

            interfaces.push(cached_class);
        }

        Ok(interfaces)
    }

    fn common_superclass<'other_local_1: 'local, 'other_local_2: 'local, T, U>(
        &mut self,
        class1: T,
        class2: U,
    ) -> Result<GlobalRef>
    where
        T: Desc<'local, JClass<'other_local_1>>,
        U: Desc<'local, JClass<'other_local_2>>,
    {
        let class1 = class1.lookup(self)?;
        let class2 = class2.lookup(self)?;
        let class1: &JClass = class1.as_ref();
        let class2: &JClass = class2.as_ref();
        let mut cls1 = jclass_from_instance(self, class1)?;
        let cls2 = jclass_from_instance(self, class2)?;

        if self.is_assignable_from(&cls2, &cls1)? {
            return Ok(cls1);
        }

        if self.is_assignable_from(&cls1, &cls2)? {
            return Ok(cls2);
        }

        if self.is_interface(&cls1)? || self.is_interface(&cls2)? {
            return jclass(self, OBJECT_CLASS_PATH);
        }

        while {
            cls1 = match self.lookup_superclass(&cls1)? {
                Some(cls) => cls,
                None => return jclass(self, OBJECT_CLASS_PATH),
            };

            !self.is_assignable_from(&cls2, &cls1)?
        } {}

        let cls1: &JClass = cls1.as_obj().as_ref().into();

        jclass_from_instance(self, cls1)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        class::{class_cache, HierExt},
        errors::HierResult,
        jni_env,
    };

    #[test]
    fn test_lookup_caching() -> HierResult<()> {
        let mut env = jni_env();
        let _class1 = env.lookup_class("java/lang/Object")?;

        // We can't actually expect how many classes has been look up
        // assert_eq!(class_cache().lock().unwrap().len(), 1);

        env.free_lookup()?;

        assert_eq!(class_cache().read()?.len(), 0);

        Ok(())
    }

    #[test]
    fn test_number_common_super_class() -> HierResult<()> {
        let mut env = jni_env();
        let class1 = env.lookup_class("java/lang/Integer")?;
        let class2 = env.lookup_class("java/lang/Float")?;
        let superclass = env.common_superclass(&class1, &class2)?;
        assert_eq!("java/lang/Number", env.class_name(&superclass)?);

        Ok(())
    }

    #[test]
    #[cfg_attr(
        not(any(jvm_v8, jvm_v11, jvm_v17, jvm_v21)),
        ignore = "No Java LTS version provided"
    )]
    /// Tests all implemented interfaces on `java/lang/Integer`
    /// (non recursively to super class which is `java/lang/Number`)
    fn test_interfaces() -> HierResult<()> {
        let implemented_interfaces = if cfg!(any(jvm_v8, jvm_v11)) {
            vec!["java/lang/Comparable"]
        } else if cfg!(any(jvm_v17, jvm_v21)) {
            vec![
                "java/lang/Comparable",
                "java/lang/constant/Constable",
                "java/lang/constant/ConstantDesc",
            ]
        } else {
            unreachable!()
        };

        let mut env = jni_env();
        let class = env.lookup_class("java/lang/Integer")?;
        let interfaces = env.interfaces(&class)?;
        let interface_names = interfaces
            .iter()
            .map(|interface| env.class_name(interface))
            .collect::<HierResult<Vec<_>>>()?;

        assert_eq!(implemented_interfaces, interface_names);

        Ok(())
    }
}
