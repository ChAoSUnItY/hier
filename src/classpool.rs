use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use jni::{
    objects::{JClass, JString, JValueGen},
    signature::{JavaType, ReturnType},
    JNIEnv,
};

use crate::{
    bridge::jni_env,
    class::{Class, ClassInternal},
    classpath::ClassPath,
};
use crate::{
    classpath::{DESC_TO_WRAPPER_CLASS_CP, PRIMITIVE_TYPES_TO_DESC},
    errors::HierResult as Result,
};

type ClassCache = HashMap<String, Arc<Mutex<ClassInternal>>>;

pub struct ClassPool<'local> {
    jni_env: JNIEnv<'local>,
    class_cache: ClassCache,
}

impl<'local> ClassPool<'local> {
    /// Constructs a new [`ClassPool`] by invoking a new [`JavaVM`](jni::JavaVM) and
    /// attaches its [`JNIEnv`] from permanently.
    /// 
    /// When you are interacting with JNI manually (e.g. calling from Java side),
    /// consider use [`from_exist_env`](Self::from_exist_env).
    pub fn from_permanent_env() -> Result<Self> {
        jni_env().map(|env| Self::from_exist_env(&env))
    }

    /// Constructs a new [`ClassPool`] by cloning existed [`JNIEnv`].
    pub fn from_exist_env(jni_env: &JNIEnv<'local>) -> Self {
        Self {
            jni_env: unsafe { jni_env.unsafe_clone() },
            class_cache: HashMap::new(),
        }
    }

    /// Lookups a class, either from [`ClassPool`]'s internal class cache if exists, or
    /// find given class from JNI and caches.
    /// 
    /// # Class path syntax
    /// 
    /// [`lookup_class`](Self::lookup_class) uses `java.lang.Class#forName`'s class path
    /// syntax, e.g. `java.lang.Object`, instead of JNI's class path `java/lang/Object`.
    /// 
    /// # Exceptions
    /// 
    /// If lookups a single or multiple dimension `void` type array, JVM will throws an 
    /// exception and this function will return an [`Err`].
    pub fn lookup_class<CP>(&mut self, class_path: CP) -> Result<Class>
    where
        CP: Into<ClassPath>,
    {
        let class_path: String = class_path.into().as_jni().into();

        self.fetch_class(&class_path).map(Class::new)
    }

    /// Gets the internal class cache's size.
    pub fn len(&self) -> usize {
        self.class_cache.len()
    }

    /// Determines if the internal class cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fetch an [GlobalRef] (JClass) either from cache if already fetched before, or directly
    /// from JNI interface if not. After each successful fetching operation, [GlobalRef] (JClass)
    /// instance will exist until the termination of program, if this is not desired,
    /// use [free_jclass_cache] to free cache.
    pub(crate) fn fetch_class(&mut self, class_path: &str) -> Result<Arc<Mutex<ClassInternal>>> {
        if let Some(cached_class) = self.class_cache.get(class_path) {
            Ok(cached_class.clone())
        } else if PRIMITIVE_TYPES_TO_DESC.contains_key(class_path) {
            self.fetch_primitive_class(class_path)
        } else {
            let jclass = self.jni_env.find_class(class_path)?;
            self.fetch_class_from_jclass(&jclass, Some(class_path))
        }
    }

    pub(crate) fn fetch_class_from_jclass(
        &mut self,
        jclass: &JClass<'_>,
        known_jclass_cp: Option<&str>,
    ) -> Result<Arc<Mutex<ClassInternal>>> {
        match known_jclass_cp {
            Some(cp) => self.fetch_class_from_jclass_internal(jclass, cp),
            None => {
                let cp = self.jni_env.with_local_frame(1, |env| {
                    let method_id = env.get_method_id(
                        ClassInternal::CLASS_JNI_CP,
                        "getName",
                        "()Ljava/lang/String;",
                    )?;
                    let class_name: JString = unsafe {
                        env.call_method_unchecked(jclass, method_id, ReturnType::Object, &[])
                            .and_then(JValueGen::l)
                            .map(Into::into)?
                    };

                    unsafe {
                        env.get_string_unchecked(&class_name)
                            .map(Into::<String>::into)
                    }
                })?;

                self.fetch_class_from_jclass_internal(jclass, &cp)
            }
        }
    }

    fn fetch_class_from_jclass_internal(
        &mut self,
        jclass: &JClass<'_>,
        known_jclass_cp: &str,
    ) -> Result<Arc<Mutex<ClassInternal>>> {
        let glob_ref = self.jni_env.new_global_ref(jclass)?;
        let class = Arc::new(Mutex::new(ClassInternal::new(glob_ref)));

        Ok(self
            .class_cache
            .entry(known_jclass_cp.to_string())
            .or_insert(class)
            .clone())
    }

    fn fetch_primitive_class(&mut self, primitive_name: &str) -> Result<Arc<Mutex<ClassInternal>>> {
        let wrapper_class_cp = PRIMITIVE_TYPES_TO_DESC
            .get(primitive_name)
            .and_then(|desc| DESC_TO_WRAPPER_CLASS_CP.get(desc))
            .unwrap();
        let static_field_id = self.jni_env.get_static_field_id(
            wrapper_class_cp,
            "TYPE",
            format!("L{};", ClassInternal::CLASS_JNI_CP),
        )?;
        let wrapper_class: JClass = self
            .jni_env
            .get_static_field_unchecked(
                wrapper_class_cp,
                static_field_id,
                JavaType::Object(ClassInternal::CLASS_JNI_CP.to_string()),
            )
            .and_then(JValueGen::l)?
            .into();

        self.fetch_class_from_jclass_internal(&wrapper_class, primitive_name)
    }
}

impl<'local> Deref for ClassPool<'local> {
    type Target = JNIEnv<'local>;

    fn deref(&self) -> &Self::Target {
        &self.jni_env
    }
}

impl<'local> DerefMut for ClassPool<'local> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.jni_env
    }
}
