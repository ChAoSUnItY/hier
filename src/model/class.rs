use std::fmt::{Display, Pointer};
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};

use jni::objects::{GlobalRef, JObject, JObjectArray, JString, JValue, JValueGen, JValueOwned};
use jni::signature::{Primitive, ReturnType};
use once_cell::sync::OnceCell;

use crate::classpool::ClassPool;
use crate::errors::HierResult as Result;
use crate::modifiers::Modifiers;

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
    pub fn superclass(&mut self, cp: &mut ClassPool<'_>) -> Result<Option<Self>> {
        let mut class = self.lock()?;
        class
            .superclass(cp)
            .map(|opt_superclass| opt_superclass.map(Self::new))
    }

    /// Fetches class name.
    ///
    /// This function is equivalent to `java.lang.Class#getName`.
    // TODO: Distinct other naming fetching functions
    pub fn name(&mut self, cp: &mut ClassPool<'_>) -> Result<String> {
        let mut class = self.lock()?;
        class.name(cp)
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
    pub fn modifiers(&mut self, cp: &mut ClassPool<'_>) -> Result<u16> {
        let mut class = self.lock()?;
        class.modifiers(cp)
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
    pub fn interfaces(&mut self, cp: &mut ClassPool<'_>) -> Result<Vec<Self>> {
        let mut class = self.lock()?;
        class
            .interfaces(cp)
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
    pub fn is_assignable_from(&mut self, cp: &mut ClassPool<'_>, other: &Self) -> Result<bool> {
        let mut class = self.lock()?;
        let other = other.lock()?;
        class.is_assignable_from(cp, &other)
    }

    /// Determines if the class is an interface.
    pub fn is_interface(&mut self, cp: &mut ClassPool<'_>) -> Result<bool> {
        let mut class = self.lock()?;
        class.is_interface(cp)
    }

    /// Determines if the class is an annotation interface.
    pub fn is_annotation(&mut self, cp: &mut ClassPool<'_>) -> Result<bool> {
        let mut class = self.lock()?;
        class.is_annotation(cp)
    }

    /// Determines if the class has synthetic modifier bit set.
    pub fn is_synthetic(&mut self, cp: &mut ClassPool<'_>) -> Result<bool> {
        let mut class = self.lock()?;
        class.is_synthetic(cp)
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
            superclass: OnceCell::new(),
            inner: class_obj,
            class_name: OnceCell::new(),
            modifiers: OnceCell::new(),
            interfaces: OnceCell::new(),
        }
    }

    fn superclass(&mut self, cp: &mut ClassPool<'_>) -> Result<Option<Arc<Mutex<Self>>>> {
        self.superclass
            .get_or_try_init(|| {
                let Some(superclass) = cp.get_superclass(&self.inner)? else {
                    return Ok(None);
                };
                let cached_superclass = cp.fetch_class_from_jclass(&superclass, None)?;

                Ok(Some(Arc::downgrade(&cached_superclass)))
            })
            .map(Option::as_ref)
            .map(|opt_superclass| opt_superclass.and_then(Weak::upgrade))
    }

    fn name(&mut self, cp: &mut ClassPool<'_>) -> Result<String> {
        self.class_name
            .get_or_try_init(|| {
                cp.push_local_frame(1)?;

                let method_id =
                    cp.get_method_id(Self::CLASS_JNI_CP, "getName", "()Ljava/lang/String;")?;
                let class_name: JString = unsafe {
                    cp.call_method_unchecked(&self.inner, method_id, ReturnType::Object, &[])
                        .and_then(JValueGen::l)
                        .map(Into::into)?
                };

                let string = unsafe {
                    cp.get_string_unchecked(&class_name)
                        .map(Into::<String>::into)
                };

                unsafe {
                    cp.pop_local_frame(&JObject::null())?;
                }

                string
            })
            .cloned()
            .map_err(Into::into)
    }

    fn modifiers(&mut self, cp: &mut ClassPool<'_>) -> Result<u16> {
        self.modifiers
            .get_or_try_init(|| {
                let method_id = cp.get_method_id(Self::CLASS_JNI_CP, "getModifiers", "()I")?;

                unsafe {
                    cp.call_method_unchecked(
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

    fn interfaces(&mut self, cp: &mut ClassPool<'_>) -> Result<&Vec<Arc<Mutex<Self>>>> {
        self.interfaces.get_or_try_init(|| {
            cp.push_local_frame(1)?;
            let method_id =
                cp.get_method_id(Self::CLASS_JNI_CP, "getInterfaces", "()[Ljava/lang/Class;")?;
            let interface_arr: JObjectArray = unsafe {
                cp.call_method_unchecked(&self.inner, method_id, ReturnType::Array, &[])
                    .and_then(JValueGen::l)?
                    .into()
            };
            let interfaces_len = cp.get_array_length(&interface_arr)?;
            let mut interfaces = Vec::with_capacity(interfaces_len as usize);

            for i in 0..interfaces_len {
                let interface_class = cp.get_object_array_element(&interface_arr, i)?.into();
                let interface_class = cp.fetch_class_from_jclass(&interface_class, None)?;

                interfaces.push(interface_class);
            }

            unsafe {
                cp.pop_local_frame(&JObject::null())?;
            }

            Ok(interfaces)
        })
    }

    fn is_assignable_from(&mut self, cp: &mut ClassPool<'_>, other: &Self) -> Result<bool> {
        // FIXME: Should we explore the both classes class hierarchy and so the
        // whole hierarchy tree can be cached and used later for better performance?
        let method_id = cp.get_method_id(
            Self::CLASS_JNI_CP,
            "isAssignableFrom",
            "(Ljava/lang/Class;)Z",
        )?;

        unsafe {
            cp.call_method_unchecked(
                &self.inner,
                method_id,
                ReturnType::Primitive(Primitive::Boolean),
                &[Into::<JValue>::into(&other.inner).as_jni()],
            )
            .and_then(JValueOwned::z)
            .map_err(Into::into)
        }
    }

    fn is_interface(&mut self, cp: &mut ClassPool<'_>) -> Result<bool> {
        self.modifiers(cp).map(Modifiers::is_interface_bits)
    }

    fn is_annotation(&mut self, cp: &mut ClassPool<'_>) -> Result<bool> {
        self.modifiers(cp).map(Modifiers::is_annotation_bits)
    }

    fn is_synthetic(&mut self, cp: &mut ClassPool<'_>) -> Result<bool> {
        self.modifiers(cp).map(Modifiers::is_synthetic_bits)
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

#[cfg(all(test, feature = "invocation"))]
mod test {
    use rstest::rstest;
    use serial_test::serial;

    use crate::{class::Class, classpool::ClassPool, errors::HierResult};

    #[test]
    #[serial]
    fn test_lookup_caching() -> HierResult<()> {
        let mut cp = ClassPool::from_permanent_env()?;
        let _class1 = cp.lookup_class("java.lang.Object")?;

        assert_eq!(cp.len(), 1);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_superclass() -> HierResult<()> {
        let mut cp = ClassPool::from_permanent_env()?;
        let mut class = cp.lookup_class("java.lang.Integer")?;
        let superclass = class.superclass(&mut cp)?;

        assert!(superclass.is_some());

        let mut superclass = superclass.unwrap();

        assert_eq!(superclass.name(&mut cp)?, "java.lang.Number");

        Ok(())
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
        let mut cp = ClassPool::from_permanent_env()?;

        assert_eq!(cp.lookup_class(input)?.name(&mut cp)?, get_name_result);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_unsupported_class_name() -> HierResult<()> {
        let mut cp = ClassPool::from_permanent_env()?;

        assert!(cp.lookup_class("void[]").is_err());

        Ok(())
    }

    #[test]
    #[serial]
    fn test_is_assignable_from() -> HierResult<()> {
        let mut cp = ClassPool::from_permanent_env()?;
        let mut class1 = cp.lookup_class("java.lang.Integer")?;
        let superclass = class1.superclass(&mut cp)?;

        assert!(superclass.is_some());

        let mut superclass = superclass.unwrap();

        assert!(superclass.is_assignable_from(&mut cp, &class1)?);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_is_interface() -> HierResult<()> {
        let mut cp = ClassPool::from_permanent_env()?;
        let mut interface = cp.lookup_class("java.lang.Comparable")?;

        assert!(interface.is_interface(&mut cp)?);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_is_annotation() -> HierResult<()> {
        let mut env = ClassPool::from_permanent_env()?;
        let mut annotation = env.lookup_class("java.lang.Override")?;

        assert!(annotation.is_annotation(&mut env)?);

        Ok(())
    }

    #[rstest]
    #[case("java.lang.Integer", "java.lang.Float", "java.lang.Number")]
    #[case("java.util.EnumMap", "java.util.HashMap", "java.util.AbstractMap")]
    #[serial]
    fn test_common_superclass(
        #[case] class1: &'static str,
        #[case] class2: &'static str,
        #[case] common_superclass_name: &'static str,
    ) -> HierResult<()> {
        fn find_most_common_superclass(
            cp: &mut ClassPool,
            class1: &mut Class,
            class2: &mut Class,
        ) -> HierResult<Class> {
            if class2.is_assignable_from(cp, class1)? {
                return Ok(class1.clone());
            }

            if class1.is_assignable_from(cp, class2)? {
                return Ok(class2.clone());
            }

            if class1.is_interface(cp)? || class2.is_interface(cp)? {
                return cp.lookup_class("java.lang.Object");
            }

            let mut cls1 = class1.clone();
            while {
                cls1 = match cls1.superclass(cp)? {
                    Some(superclass) => superclass,
                    None => return Ok(cls1),
                };

                !cls1.is_assignable_from(cp, class2)?
            } {}

            Ok(cls1)
        }

        let mut cp = ClassPool::from_permanent_env()?;
        let mut class1 = cp.lookup_class(class1)?;
        let mut class2 = cp.lookup_class(class2)?;
        let mut common_superclass = find_most_common_superclass(&mut cp, &mut class1, &mut class2)?;

        assert_eq!(common_superclass.name(&mut cp)?, common_superclass_name);

        Ok(())
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

        let mut cp = ClassPool::from_permanent_env()?;
        let mut class = cp.lookup_class("java.lang.Integer")?;
        let mut interfaces = class.interfaces(&mut cp)?;
        let interface_names = interfaces
            .iter_mut()
            .map(|interface| interface.name(&mut cp))
            .collect::<HierResult<Vec<_>>>()?;

        assert_eq!(interface_names, implemented_interfaces);

        Ok(())
    }
}
