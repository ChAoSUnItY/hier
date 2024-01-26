#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex, OnceLock},
};

use class::{Class, ClassInternal};
use classpath::{ClassPath, DESC_TO_WRAPPER_CLASS_CP, PRIMITIVE_TYPES_TO_DESC};
use errors::HeirResult as Result;
use jni::{
    descriptors::Desc,
    objects::{JClass, JValueGen},
    signature::{JavaType, ReturnType},
    InitArgsBuilder, JNIEnv, JNIVersion, JavaVM,
};
use once_cell::sync::OnceCell;
use version::JavaVersion;

pub mod classpath;
pub mod errors;
#[cfg(feature = "graph")]
pub mod graph;
pub mod version;

mod model {
    pub mod class;
    pub mod modifiers;
}

pub use model::*;

pub extern crate jni;

/// Get JVM instance, initialize if does not exist.
fn jvm() -> Result<&'static Arc<JavaVM>> {
    static JVM: OnceCell<Arc<JavaVM>> = OnceCell::new();

    JVM.get_or_try_init(|| -> Result<Arc<JavaVM>> {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .build()?;

        let jvm = JavaVM::new(jvm_args)?;

        Ok(Arc::new(jvm))
    })
}

/// Get JNI environment instance, notice that the thread is attached permanently.
pub fn jni_env() -> Result<JNIEnv<'static>> {
    jvm().and_then(|jvm| jvm.attach_current_thread_permanently().map_err(Into::into))
}

pub type ClassCache = HashMap<String, Arc<Mutex<ClassInternal>>>;

fn class_cache() -> &'static Mutex<ClassCache> {
    static CLASS_CACHE: OnceLock<Mutex<ClassCache>> = OnceLock::new();
    CLASS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fetch an [GlobalRef] (JClass) either from cache if already fetched before, or directly
/// from JNI interface if not. After each successful fetching operation, [GlobalRef] (JClass)
/// instance will exist until the termination of program, if this is not desired,
/// use [free_jclass_cache] to free cache.
fn fetch_class<'local>(
    env: &mut JNIEnv<'local>,
    class_path: &str,
) -> Result<Arc<Mutex<ClassInternal>>> {
    let cache = class_cache().lock()?;

    if let Some(cached_class) = cache.get(class_path) {
        Ok(cached_class.clone())
    } else {
        drop(cache);
        if PRIMITIVE_TYPES_TO_DESC.contains_key(class_path) {
            fetch_primitive_class(env, class_path)
        } else {
            let jclass = env.find_class(class_path)?;
            fetch_class_from_jclass(env, &jclass, Some(class_path))
        }
    }
}

fn fetch_class_from_jclass<'local, 'other_local, 'str>(
    env: &mut JNIEnv<'local>,
    jclass: &JClass<'other_local>,
    known_jclass_cp: Option<&'str str>,
) -> Result<Arc<Mutex<ClassInternal>>> {
    match known_jclass_cp {
        Some(cp) => fetch_class_from_jclass_internal(env, jclass, cp),
        None => {
            let method_id = env.get_method_id(
                ClassInternal::CLASS_JNI_CP,
                "getName",
                "()Ljava/lang/String;",
            )?;
            let class_name = unsafe {
                env.call_method_unchecked(jclass, method_id, ReturnType::Object, &[])
                    .and_then(JValueGen::l)?
            };
            let class_name = env.auto_local(class_name);
            let cp = unsafe {
                env.get_string_unchecked(class_name.deref().into())
                    .map(Into::<String>::into)?
            };

            fetch_class_from_jclass_internal(env, jclass, &cp)
        }
    }
}

fn fetch_class_from_jclass_internal<'local, 'other_local, 'str>(
    env: &mut JNIEnv<'local>,
    jclass: &JClass<'other_local>,
    known_jclass_cp: &'str str,
) -> Result<Arc<Mutex<ClassInternal>>> {
    let mut cache = class_cache().lock()?;
    let glob_ref = env.new_global_ref(jclass)?;
    let class = Arc::new(Mutex::new(ClassInternal::new(glob_ref)));

    Ok(cache
        .entry(known_jclass_cp.to_string())
        .or_insert(class)
        .clone())
}

fn fetch_primitive_class<'local>(
    env: &mut JNIEnv<'local>,
    primitive_name: &str,
) -> Result<Arc<Mutex<ClassInternal>>> {
    let wrapper_class_cp = PRIMITIVE_TYPES_TO_DESC
        .get(primitive_name)
        .and_then(|desc| DESC_TO_WRAPPER_CLASS_CP.get(desc))
        .unwrap();
    let static_field_id = env.get_static_field_id(
        wrapper_class_cp,
        "TYPE",
        format!("L{};", ClassInternal::CLASS_JNI_CP),
    )?;
    let wrapper_class: JClass = env
        .get_static_field_unchecked(
            wrapper_class_cp,
            static_field_id,
            JavaType::Object(ClassInternal::CLASS_JNI_CP.to_string()),
        )
        .and_then(JValueGen::l)?
        .into();

    fetch_class_from_jclass_internal(env, &wrapper_class, primitive_name)
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
    fn lookup_class<CP>(&mut self, class_path: CP) -> Result<Class>
    where
        CP: Into<ClassPath>;

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

    fn lookup_class<CP>(&mut self, class_path: CP) -> Result<Class>
    where
        CP: Into<ClassPath>,
    {
        let class_path: String = class_path.into().as_jni().into();

        fetch_class(self, &class_path).map(Class::new)
    }

    fn class_name<'other_local, T>(&mut self, class: T) -> Result<String>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let method_id = self.get_method_id(
            ClassInternal::CLASS_JNI_CP,
            "getName",
            "()Ljava/lang/String;",
        )?;
        let class_name = unsafe {
            self.call_method_unchecked(class.as_ref(), method_id, ReturnType::Object, &[])
                .and_then(JValueGen::l)?
        };
        let class_name = self.auto_local(class_name);

        unsafe {
            self.get_string_unchecked(class_name.deref().into())
                .map(Into::<String>::into)
                .map_err(Into::into)
        }
    }
}
