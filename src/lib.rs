#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex, OnceLock},
};

use class::{Class, ClassInternal};
use errors::HierResult as Result;
use jni::{
    descriptors::Desc,
    objects::{JClass, JValueGen},
    signature::ReturnType,
    InitArgsBuilder, JNIEnv, JNIVersion, JavaVM,
};
use once_cell::sync::OnceCell;
use version::JavaVersion;

pub mod class;
mod errors;
#[cfg(feature = "graph")]
pub mod graph;
mod modifiers;
pub mod version;

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
        let jclass = env.find_class(class_path)?;
        fetch_class_from_jclass(env, &jclass)
    }
}

fn fetch_class_from_jclass<'local, 'other_local>(
    env: &mut JNIEnv<'local>,
    jclass: &JClass<'other_local>,
) -> Result<Arc<Mutex<ClassInternal>>> {
    let jclass_cp = env.class_name(jclass)?;

    fetch_class_from_jclass_internal(env, jclass, &jclass_cp)
}

fn fetch_class_from_jclass_internal<'local, 'other_local>(
    env: &mut JNIEnv<'local>,
    jclass: &JClass<'other_local>,
    known_jclass_cp: &str,
) -> Result<Arc<Mutex<ClassInternal>>> {
    let mut cache = class_cache().lock()?;
    let glob_ref = env.new_global_ref(jclass)?;
    let class = Arc::new(Mutex::new(ClassInternal::new(glob_ref)));
    let weak_class_self_ref = Arc::downgrade(&class);
    unsafe {
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
    fn lookup_class(&mut self, class_path: &str) -> Result<Class>;

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

    fn lookup_class(&mut self, class_path: &str) -> Result<Class> {
        fetch_class(self, class_path).map(Class::new)
    }

    fn class_name<'other_local, T>(&mut self, class: T) -> Result<String>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let method_id =
            self.get_method_id(ClassInternal::CLASS_CP, "getName", "()Ljava/lang/String;")?;
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
