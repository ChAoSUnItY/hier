use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use jni::descriptors::Desc;
use jni::objects::{GlobalRef, JClass, JObjectArray, JValueGen, JValueOwned};
use jni::signature::{Primitive, ReturnType};
use jni::JNIEnv;
use once_cell::sync::OnceCell;

use crate::errors::{HierError, HierResult as Result};
use crate::modifiers::Modifiers;
use crate::version::JavaVersion;

/// A pseudo java class that projects `java/lang/Class`.
pub struct Class {
    /// A self weak reference that referenced to the [Arc] holding this
    /// [Class] instance in [class_cache]. This is guaranteed to be always
    /// upgradable unless [class_cache] is freed.
    self_cached_weak_ref: OnceCell<Weak<Mutex<Self>>>,
    inner: GlobalRef,
    superclass: OnceCell<Option<Weak<Mutex<Self>>>>,
    interfaces: OnceCell<Vec<Arc<Mutex<Self>>>>,
    class_name: OnceCell<String>,
    modifiers: OnceCell<u16>,
}

impl Class {
    pub const CLASS_CP: &'static str = "java/lang/Class";

    /// Creates new [Class] from an [GlobalRef] that stores reference to
    /// [JClass] as internal backend.
    pub fn new(class_obj: GlobalRef) -> Self {
        Self {
            self_cached_weak_ref: OnceCell::new(),
            superclass: OnceCell::new(),
            inner: class_obj,
            class_name: OnceCell::new(),
            modifiers: OnceCell::new(),
            interfaces: OnceCell::new(),
        }
    }

    /// Initialize self [Weak] reference to the own entry in [class_cache].
    /// This should be done internally and once.
    fn initialize_self_weak_ref(&mut self, weak_ref: Weak<Mutex<Self>>) {
        self.self_cached_weak_ref
            .set(weak_ref)
            .expect("self_cached_weak_ref should not be initialized yet");
    }

    /// Lookups superclass from given class instance, returns [None] for if current [Class] is
    /// `Class(java/lang/Object)` or an interface.
    pub fn superclass<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
    ) -> Result<Option<Arc<Mutex<Class>>>> {
        self.superclass
            .get_or_try_init(|| {
                let Some(superclass) = env.get_superclass(&self.inner)? else {
                    return Ok(None);
                };
                let cached_superclass = fetch_class_from_jclass(env, superclass)?;

                Ok(Some(Arc::downgrade(&cached_superclass)))
            })
            .map(|opt_superclass| {
                opt_superclass
                    .clone()
                    .and_then(|superclass| superclass.upgrade())
            })
    }

    pub fn class_name<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<&String> {
        self.class_name
            .get_or_try_init(|| {
                let method_id =
                    env.get_method_id(Self::CLASS_CP, "getName", "()Ljava/lang/String;")?;
                let class_name = unsafe {
                    env.call_method_unchecked(&self.inner, method_id, ReturnType::Object, &[])
                        .and_then(JValueGen::l)?
                };
                let class_name = env.auto_local(class_name);

                unsafe {
                    env.get_string_unchecked(class_name.deref().into())
                        .map(Into::<String>::into)
                        .map(|name| name.replace(".", "/"))
                }
            })
            .map_err(Into::into)
    }

    /// Returns class' access flags. See [Modifiers] for all possible modifiers that would
    /// OR-ing together.
    ///
    /// # Example
    ///
    /// Assuming variable class is a [Class] references to `JClass(java/lang/String)`:
    ///
    /// ```rs
    /// let mut env = jni_env()?;
    /// let class = fetch_class(&mut env, "java/lang/String")?;
    /// let modifiers = class.lock()?.modifiers()?;
    ///
    /// assert_eq!(modifiers, Modifiers::Public & Modifiers::Final)
    /// ```
    pub fn modifiers<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<u16> {
        self.modifiers
            .get_or_try_init(|| {
                let method_id = env.get_method_id(Self::CLASS_CP, "getModifiers", "()I")?;

                unsafe {
                    env.call_method_unchecked(
                        &self.inner,
                        method_id,
                        ReturnType::Primitive(Primitive::Int),
                        &[],
                    )
                    .and_then(JValueOwned::i)
                    .map(|modifiers| modifiers as u16)
                }
            })
            .map(|modifiers| *modifiers)
            .map_err(Into::into)
    }

    pub fn interfaces<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
    ) -> Result<&Vec<Arc<Mutex<Class>>>> {
        self.interfaces.get_or_try_init(|| {
            let method_id =
                env.get_method_id(Self::CLASS_CP, "getInterfaces", "()[Ljava/lang/Class;")?;
            let interface_arr: JObjectArray = unsafe {
                env.call_method_unchecked(&self.inner, method_id, ReturnType::Array, &[])
                    .and_then(JValueGen::l)?
                    .into()
            };
            let interface_arr = env.auto_local(interface_arr);
            let interfaces_len = env.get_array_length(interface_arr.deref())?;
            let mut interfaces = Vec::with_capacity(interfaces_len as usize);

            for i in 0..interfaces_len {
                let interface_class = env
                    .get_object_array_element(interface_arr.deref(), i)?
                    .into();
                let interface_class = fetch_class_from_jclass(env, interface_class)?;

                interfaces.push(interface_class);
            }

            Ok(interfaces)
        })
    }

    /// Determines if the class or interface represented by this [Class] is either the same as,
    /// or is a superclass or superinterface of, the class or interface represented by the specified
    /// [Class] parameter.
    ///
    /// If this [Class] represents primitive types, then returns true if the specified [Class] is exactly
    /// same, otherwise false.
    ///
    /// # Example
    ///
    /// ```rs
    /// let mut env = jni_env()?;
    /// let class1 = fetch_class(&mut env, "java/lang/Integer")?;
    /// let class2 = fetch_class(&mut env, "java/lang/Number")?;
    /// let is_assignable = class2.lock()?.is_assignable_from(class1.lock()?)?;
    ///
    /// assert_eq!(is_assignable, true);
    /// ```
    pub fn is_assignable_from<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        other: &Self,
    ) -> Result<bool> {
        // FIXME: Should we explore the both classes class hierarchy and so the
        // whole hierarchy tree can be cached and used later for better performance?
        env.is_assignable_from(&self.inner, &other.inner)
            .map_err(Into::into)
    }

    /// Determines if the given class is an interface type.
    pub fn is_interface<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        let modifiers = self.modifiers(env)?;

        Ok(modifiers & Modifiers::Interface == 1)
    }

    /// Returns the given 2 class' most common superclass.
    ///
    /// If 1 of the classes is interface, then returns `JClass("java/lang/Object")`.
    pub fn common_superclass<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        other: &mut Self,
    ) -> Result<Arc<Mutex<Class>>> {
        let mut cls1 = self
            .self_cached_weak_ref
            .get()
            .and_then(Weak::upgrade)
            .ok_or(HierError::DanglingClassError(format!("{:#}", self)))?;
        let cls2 = other
            .self_cached_weak_ref
            .get()
            .and_then(Weak::upgrade)
            .ok_or(HierError::DanglingClassError(format!("{:#}", other)))?;

        if other.is_assignable_from(env, self)? {
            return Ok(cls1);
        }

        if self.is_assignable_from(env, other)? {
            return Ok(cls2);
        }

        cls1 = match self.superclass(env)? {
            Some(cls) => cls,
            None => return fetch_class(env, OBJECT_CLASS_PATH),
        };

        loop {
            let mut cls1_guard = cls1.lock()?;

            if other.is_assignable_from(env, &cls1_guard)? {
                drop(cls1_guard);
                return Ok(cls1);
            }

            match cls1_guard.superclass(env)? {
                Some(cls) => {
                    drop(cls1_guard);
                    cls1 = cls;
                }
                None => return fetch_class(env, OBJECT_CLASS_PATH),
            };
        }
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Class({})",
            self.class_name.get().unwrap_or(&"...".to_owned())
        )
    }
}

pub const OBJECT_CLASS_PATH: &'static str = "java/lang/Object";

pub type ClassCache = HashMap<String, Arc<Mutex<Class>>>;

fn class_cache() -> &'static Mutex<ClassCache> {
    static CLASS_CACHE: OnceLock<Mutex<ClassCache>> = OnceLock::new();
    CLASS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fetch an [GlobalRef] (JClass) either from cache if already fetched before, or directly
/// from JNI interface if not. After each successful fetching operation, [GlobalRef] (JClass)
/// instance will exist until the termination of program, if this is not desired,
/// use [free_jclass_cache] to free cache.
fn fetch_class<'local>(env: &mut JNIEnv<'local>, class_path: &str) -> Result<Arc<Mutex<Class>>> {
    let cache = class_cache().lock()?;

    if let Some(cached_class) = cache.get(class_path) {
        Ok(cached_class.clone())
    } else {
        drop(cache);
        let jclass = env.find_class(class_path)?;
        fetch_class_from_jclass(env, jclass)
    }
}

fn fetch_class_from_jclass<'local, 'other_local>(
    env: &mut JNIEnv<'local>,
    jclass: JClass<'other_local>,
) -> Result<Arc<Mutex<Class>>> {
    let jclass_cp = env.class_name(&jclass)?;

    fetch_class_from_jclass_internal(env, jclass, &jclass_cp)
}

fn fetch_class_from_jclass_internal<'local, 'other_local>(
    env: &mut JNIEnv<'local>,
    jclass: JClass<'other_local>,
    known_jclass_cp: &str,
) -> Result<Arc<Mutex<Class>>> {
    let mut cache = class_cache().lock()?;
    let glob_ref = env.new_global_ref(jclass)?;
    let class = Arc::new(Mutex::new(Class::new(glob_ref)));
    let weak_class_self_ref = Arc::downgrade(&class);
    {
        let mut class_guard = class.lock()?;
        class_guard.initialize_self_weak_ref(weak_class_self_ref);
    }

    Ok(cache
        .entry(known_jclass_cp.to_string())
        .or_insert(class)
        .clone())
}

/// Frees jclass cache.
unsafe fn free_jclass_cache() -> Result<()> {
    class_cache().lock()?.clear();

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
    fn lookup_class(&mut self, class_path: &str) -> Result<Arc<Mutex<Class>>>;

    /// Frees the class cache.
    ///
    /// # Safety
    ///
    /// This could cause current existed unfreed [Class] to be unreliable,
    /// you'll need to get another instance of [Class] through [HierExt::lookup_class].
    ///
    /// Calling to existed unfreed [Class] would lead to undefined behaviour.
    unsafe fn free_lookup(&mut self) -> Result<()> {
        free_jclass_cache()
    }

    /// Returns the given class' class path.
    fn class_name<'other_local, T>(&mut self, class: T) -> Result<String>
    where
        T: Desc<'local, JClass<'other_local>>;
}

impl<'local> HierExt<'local> for JNIEnv<'local> {
    fn get_java_version(&mut self) -> Result<JavaVersion> {
        let sys_class = self.find_class("java/lang/System")?;
        let property = self.auto_local(self.new_string("java.specification.version")?);
        let version = self
            .call_static_method(
                sys_class,
                "getProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[(&property).into()],
            )
            .and_then(JValueGen::l)?;
        let version = self.auto_local(version);

        unsafe {
            self.get_string_unchecked(version.deref().into())
                .map(|java_str| JavaVersion::from(Into::<String>::into(java_str)))
                .map_err(Into::into)
        }
    }

    fn lookup_class(&mut self, class_path: &str) -> Result<Arc<Mutex<Class>>> {
        fetch_class(self, class_path)
    }

    fn class_name<'other_local, T>(&mut self, class: T) -> Result<String>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let method_id = self.get_method_id(Class::CLASS_CP, "getName", "()Ljava/lang/String;")?;
        let class_name = unsafe {
            self.call_method_unchecked(class.as_ref(), method_id, ReturnType::Object, &[])
                .and_then(JValueGen::l)?
        };
        let class_name = self.auto_local(class_name);

        unsafe {
            self.get_string_unchecked(class_name.deref().into())
                .map(Into::<String>::into)
                .map(|name| name.replace(".", "/"))
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    use crate::{
        class::{class_cache, HierExt},
        errors::HierResult,
        jni_env,
    };

    #[test]
    #[serial]
    fn test_lookup_caching() -> HierResult<()> {
        let mut env = jni_env()?;
        let _class1 = env.lookup_class("java/lang/Object")?;

        assert_eq!(class_cache().lock()?.len(), 1);

        unsafe {
            env.free_lookup()?;
        }

        assert_eq!(class_cache().lock()?.len(), 0);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_number_common_super_class() -> HierResult<()> {
        let mut env = jni_env()?;
        let class1 = env.lookup_class("java/lang/Integer")?;
        let class2 = env.lookup_class("java/lang/Float")?;

        {
            let mut class1 = class1.lock()?;
            let mut class2 = class2.lock()?;
            let superclass = class1.common_superclass(&mut env, &mut class2)?;
            let mut superclass_guard = superclass.lock()?;
            let superclass_name = superclass_guard.class_name(&mut env)?;

            assert_eq!("java/lang/Number", superclass_name);
        }

        unsafe {
            env.free_lookup()?;
        }

        Ok(())
    }

    #[test]
    #[serial]
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

        let mut env = jni_env()?;
        let class = env.lookup_class("java/lang/Integer")?;

        {
            let mut class = class.lock()?;
            let interfaces = class.interfaces(&mut env)?;
            let interface_names = interfaces
                .iter()
                .map(|interface| {
                    let mut interface = interface.lock()?;

                    interface.class_name(&mut env).cloned()
                })
                .collect::<HierResult<Vec<_>>>()?;

            assert_eq!(implemented_interfaces, interface_names);
        }

        unsafe {
            env.free_lookup()?;
        }

        Ok(())
    }
}
