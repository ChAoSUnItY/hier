use std::fmt::{Display, Pointer};
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};

use jni::objects::{GlobalRef, JObjectArray, JValue, JValueGen, JValueOwned};
use jni::signature::{Primitive, ReturnType};
use jni::JNIEnv;
use once_cell::sync::OnceCell;

use crate::errors::{HierError, HierResult as Result};
use crate::modifiers::Modifiers;

use crate::{fetch_class, fetch_class_from_jclass};

/// A rust side pseudo class that projects java side `java.lang.Class`, used for simplify
/// class property lookup and other class-related operations.
///
/// A [Class] is considered as a snapshot, not an realtime projected structure that always
/// syncs with java side `java.lang.Class`, which means that after internal class cache is
/// freed (See [`HierExt::free_lookup`](crate::HierExt::free_lookup)), this class is not
/// guaranteed for later operations, and should be update by fetching the latest one (See
/// [`HierExt::lookup_class`](crate::HierExt::lookup_class)). Operations after cache being
/// freed are considered undefined behavior.
#[derive(Clone)]
pub struct Class {
    inner: Arc<Mutex<ClassInternal>>,
}

impl Class {
    pub const CLASS_CP: &'static str = ClassInternal::CLASS_JNI_CP;
    pub const OBJECT_CP: &'static str = ClassInternal::OBJECT_JNI_CP;

    pub(crate) fn new(internal: Arc<Mutex<ClassInternal>>) -> Self {
        Self { inner: internal }
    }

    /// Lookups superclass from given class instance, returns [None] for if current [Class]
    /// is `Class(java.lang.Object)` or an interface.
    ///
    /// # Example
    ///
    /// ```rs
    /// let mut env = jni_env()?;
    /// let mut class = env.lookup_class("java.lang.Integer")?;
    /// let mut superclass = class.superclass(&mut env)?;
    /// let superclass_name = superclass.class_name(&mut env)?;
    ///
    /// assert_eq!(superclass_name, "java.lang.Number");
    /// ```
    pub fn superclass<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<Option<Self>> {
        let mut class = self.lock()?;
        class
            .superclass(env)
            .map(|opt_superclass| opt_superclass.map(Self::new))
    }

    /// Fetches class name.
    ///
    /// This function is equivalent to `java.lang.Class#getName`.
    // TODO: Distinct other naming fetching functions
    pub fn name<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<String> {
        let mut class = self.lock()?;
        class.name(env)
    }

    /// Returns class' access flags. See [Modifiers] for all possible modifiers that would
    /// OR-ing together.
    ///
    /// # Example
    ///
    /// ```rs
    /// let mut env = jni_env()?;
    /// let mut class = env.lookup_class("java.lang.String")?;
    /// let modifiers = class.modifiers(&mut env)?;
    ///
    /// assert_eq!(modifiers, Modifiers::Public & Modifiers::Final)
    /// ```
    pub fn modifiers<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<u16> {
        let mut class = self.lock()?;
        class.modifiers(env)
    }

    /// Returns array of interface [Class] that represents the interfaces implemented by
    /// current [Class].
    ///
    /// If current [Class] represents a class, then returns all interfaces implemented by
    /// this class.
    ///
    /// If current [Class] represents a interface, then returns all interfaces extended by
    /// this interface.
    ///
    /// If current [Class] represents a primitive type or void, or either class or interface
    /// doesn't implemented or extended any interfaces, then returns empty [Vec].
    ///
    /// The order of interfaces always corresponds to original clause of declaration.
    ///
    /// # Example
    ///
    /// ```rs
    /// let mut env = jni_env();
    /// let mut class = env.lookup_class("java.lang.Integer")?;
    /// let mut interfaces = class.interfaces(&mut env)?;
    /// let interface_names = interfaces
    ///     .iter_mut()
    ///     .map(|interface| interface.class_name(&mut env))
    ///     .collect::<Result<Vec<_>, HierError>>()
    ///
    /// println!("{interface_names:#}");
    /// ```
    pub fn interfaces<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<Vec<Self>> {
        let mut class = self.lock()?;
        class
            .interfaces(env)
            .map(|interfaces| interfaces.iter().map(Arc::clone).map(Class::new).collect())
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
    /// let mut class1 = env.lookup_class("java.lang.Integer")?;
    /// let mut class2 = env.lookup_class("java.lang.Number")?;
    /// let is_assignable = class2.is_assignable_from(&mut class1)?;
    ///
    /// assert_eq!(is_assignable, true);
    /// ```
    pub fn is_assignable_from<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        other: &Self,
    ) -> Result<bool> {
        let mut class = self.lock()?;
        let other = other.lock()?;
        class.is_assignable_from(env, &other)
    }

    /// Determines if the class is an interface.
    pub fn is_interface<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        let mut class = self.lock()?;
        class.is_interface(env)
    }

    /// Determines if the class is an annotation interface.
    pub fn is_annotation<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        let mut class = self.lock()?;
        class.is_annotation(env)
    }

    /// Determines if the class has synthetic modifier bit set.
    pub fn is_synthetic<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        let mut class = self.lock()?;
        class.is_synthetic(env)
    }

    /// Returns the 2 class' most common superclass.
    ///
    /// If 1 of the classes is interface, then returns `JClass("java.lang.Object")`.
    ///
    /// # Example
    ///
    /// ```rs
    /// ```
    #[deprecated = "Not part of java.lang.Class, this will be moved to example to demonstrate some internal interactions"]
    pub fn common_superclass<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        other: &mut Self,
    ) -> Result<Class> {
        let mut class = self.lock()?;
        let mut other = other.lock()?;
        class.common_superclass(env, &mut other).map(Class::new)
    }
}

impl Deref for Class {
    type Target = Arc<Mutex<ClassInternal>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// A pseudo java class that projects `java.lang.Class`.
pub struct ClassInternal {
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

impl ClassInternal {
    pub(crate) const CLASS_JNI_CP: &'static str = "java/lang/Class";
    pub(crate) const OBJECT_JNI_CP: &'static str = "java/lang/Object";

    /// Creates new [Class] from an [GlobalRef] that stores reference to
    /// [JClass] as internal backend.
    pub(crate) fn new(class_obj: GlobalRef) -> Self {
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
    pub(crate) unsafe fn initialize_self_weak_ref(&mut self, weak_ref: Weak<Mutex<Self>>) {
        self.self_cached_weak_ref
            .set(weak_ref)
            .expect("self_cached_weak_ref should not be initialized yet");
    }

    fn superclass<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<Option<Arc<Mutex<Self>>>> {
        self.superclass
            .get_or_try_init(|| {
                let Some(superclass) = env.get_superclass(&self.inner)? else {
                    return Ok(None);
                };
                let cached_superclass = fetch_class_from_jclass(env, &superclass)?;

                Ok(Some(Arc::downgrade(&cached_superclass)))
            })
            .map(Option::as_ref)
            .map(|opt_superclass| opt_superclass.and_then(Weak::upgrade))
    }

    fn name<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<String> {
        self.class_name
            .get_or_try_init(|| {
                let method_id =
                    env.get_method_id(Self::CLASS_JNI_CP, "getName", "()Ljava/lang/String;")?;
                let class_name = unsafe {
                    env.call_method_unchecked(&self.inner, method_id, ReturnType::Object, &[])
                        .and_then(JValueGen::l)?
                };
                let class_name = env.auto_local(class_name);

                unsafe {
                    env.get_string_unchecked(class_name.deref().into())
                        .map(Into::<String>::into)
                }
            })
            .cloned()
            .map_err(Into::into)
    }

    fn modifiers<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<u16> {
        self.modifiers
            .get_or_try_init(|| {
                let method_id = env.get_method_id(Self::CLASS_JNI_CP, "getModifiers", "()I")?;

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
            .copied()
            .map_err(Into::into)
    }

    fn interfaces<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<&Vec<Arc<Mutex<Self>>>> {
        self.interfaces.get_or_try_init(|| {
            let method_id =
                env.get_method_id(Self::CLASS_JNI_CP, "getInterfaces", "()[Ljava/lang/Class;")?;
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
                let interface_class = fetch_class_from_jclass(env, &interface_class)?;

                interfaces.push(interface_class);
            }

            Ok(interfaces)
        })
    }

    fn is_assignable_from<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        other: &Self,
    ) -> Result<bool> {
        // FIXME: Should we explore the both classes class hierarchy and so the
        // whole hierarchy tree can be cached and used later for better performance?
        let method_id = env.get_method_id(
            Self::CLASS_JNI_CP,
            "isAssignableFrom",
            "(Ljava/lang/Class;)Z",
        )?;
        unsafe {
            env.call_method_unchecked(
                &self.inner,
                method_id,
                ReturnType::Primitive(Primitive::Boolean),
                &[Into::<JValue>::into(&other.inner).as_jni()],
            )
            .and_then(JValueOwned::z)
            .map_err(Into::into)
        }
    }

    fn is_interface<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        self.modifiers(env).map(Modifiers::is_interface_bits)
    }

    fn is_annotation<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        self.modifiers(env).map(Modifiers::is_annotation_bits)
    }

    fn is_synthetic<'local>(&mut self, env: &mut JNIEnv<'local>) -> Result<bool> {
        self.modifiers(env).map(Modifiers::is_synthetic_bits)
    }

    #[deprecated = "Not part of java.lang.Class, this will be moved to example to demonstrate some internal interactions"]
    fn common_superclass<'local>(
        &mut self,
        env: &mut JNIEnv<'local>,
        other: &mut Self,
    ) -> Result<Arc<Mutex<Self>>> {
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
            None => return fetch_class(env, Self::OBJECT_JNI_CP),
        };

        loop {
            let mut cls1_guard = cls1.lock()?;

            if !other.is_assignable_from(env, &cls1_guard)? {
                drop(cls1_guard);
                return Ok(cls1);
            }

            match cls1_guard.superclass(env)? {
                Some(cls) => {
                    drop(cls1_guard);
                    cls1 = cls;
                }
                None => return fetch_class(env, Self::OBJECT_JNI_CP),
            };
        }
    }
}

impl Display for ClassInternal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Class({})",
            self.class_name.get().unwrap_or(&"...".to_owned())
        )
    }
}

#[cfg(test)]
mod test {
    use jni::JNIEnv;
    use rstest::rstest;
    use serial_test::serial;

    use crate::{class_cache, errors::HierResult, jni_env, HierExt};

    fn free_lookup<'local>(env: &mut JNIEnv<'local>) -> HierResult<()> {
        unsafe { env.free_lookup() }
    }

    #[test]
    #[serial]
    fn test_lookup_caching() -> HierResult<()> {
        let mut env = jni_env()?;
        let _class1 = env.lookup_class("java.lang.Object")?;

        assert_eq!(class_cache().lock()?.len(), 1);

        free_lookup(&mut env)?;

        assert_eq!(class_cache().lock()?.len(), 0);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_superclass() -> HierResult<()> {
        let mut env = jni_env()?;
        let mut class = env.lookup_class("java.lang.Integer")?;
        let superclass = class.superclass(&mut env)?;

        assert!(superclass.is_some());

        let mut superclass = superclass.unwrap();

        assert_eq!(superclass.name(&mut env)?, "java.lang.Number");

        free_lookup(&mut env)
    }

    #[rstest]
    #[case("void", "void")]
    #[case("int", "int")]
    #[case("int[]", "[I")]
    #[case("java.lang.Class", "java.lang.Class")]
    #[case("java.lang.Class[]", "[Ljava.lang.Class;")]
    #[case("java.util.Map$Entry", "java.util.Map$Entry")]
    #[serial]
    fn test_class_name(
        #[case] input: &'static str,
        #[case] get_name_result: &'static str,
    ) -> HierResult<()> {
        let mut env = jni_env()?;

        assert_eq!(env.lookup_class(input)?.name(&mut env)?, get_name_result);

        free_lookup(&mut env)
    }

    #[test]
    #[serial]
    fn test_is_assignable_from() -> HierResult<()> {
        let mut env = jni_env()?;
        let mut class1 = env.lookup_class("java.lang.Integer")?;
        let superclass = class1.superclass(&mut env)?;

        assert!(superclass.is_some());

        let mut superclass = superclass.unwrap();

        assert!(superclass.is_assignable_from(&mut env, &class1)?);

        free_lookup(&mut env)
    }

    #[test]
    #[serial]
    fn test_is_interface() -> HierResult<()> {
        let mut env = jni_env()?;
        let mut interface = env.lookup_class("java.lang.Comparable")?;

        assert!(interface.is_interface(&mut env)?);

        free_lookup(&mut env)
    }

    #[test]
    #[serial]
    fn test_is_annotation() -> HierResult<()> {
        let mut env = jni_env()?;
        let mut annotation = env.lookup_class("java.lang.Override")?;

        assert!(annotation.is_annotation(&mut env)?);

        free_lookup(&mut env)
    }

    #[test]
    #[serial]
    fn test_number_common_super_class() -> HierResult<()> {
        let mut env = jni_env()?;
        let mut class1 = env.lookup_class("java.lang.Integer")?;
        let mut class2 = env.lookup_class("java.lang.Float")?;
        let mut common_superclass = class1.common_superclass(&mut env, &mut class2)?;

        assert_eq!(common_superclass.name(&mut env)?, "java.lang.Number");

        free_lookup(&mut env)
    }

    #[test]
    #[serial]
    #[cfg_attr(
        not(any(jvm_v8, jvm_v11, jvm_v17, jvm_v21)),
        ignore = "No Java LTS version provided"
    )]
    /// Tests all implemented interfaces on `java.lang.Integer`
    /// (non recursively to super class which is `java.lang.Number`)
    fn test_interfaces() -> HierResult<()> {
        let implemented_interfaces = if cfg!(any(jvm_v8, jvm_v11)) {
            vec!["java.lang.Comparable"]
        } else if cfg!(any(jvm_v17, jvm_v21)) {
            vec![
                "java.lang.Comparable",
                "java.lang.constant.Constable",
                "java.lang.constant.ConstantDesc",
            ]
        } else {
            unreachable!()
        };

        let mut env = jni_env()?;
        let mut class = env.lookup_class("java.lang.Integer")?;
        let mut interfaces = class.interfaces(&mut env)?;
        let interface_names = interfaces
            .iter_mut()
            .map(|interface| interface.name(&mut env))
            .collect::<HierResult<Vec<_>>>()?;

        assert_eq!(interface_names, implemented_interfaces);

        free_lookup(&mut env)
    }
}
