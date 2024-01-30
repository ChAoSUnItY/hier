#![doc = include_str!("../README.md")]

use std::ops::Deref;

use class::ClassInternal;

use errors::HierResult as Result;
use jni::{
    descriptors::Desc,
    objects::{JClass, JValueGen},
    signature::ReturnType,
    JNIEnv,
};
use version::JavaVersion;

pub mod classpath;
pub mod classpool;
pub mod errors;
#[cfg(feature = "graph")]
pub mod graph;
#[cfg(feature = "invocation")]
mod java_vm;
pub mod version;

mod model {
    pub mod class;
    pub mod modifiers;
}

pub use model::*;

pub extern crate jni;

/// The additional definition for [JNIEnv], used for define
/// [JClass] caching (see [HierExt::lookup_class] and [HierExt::free_lookup])
/// and other useful class-related functions.
pub trait HierExt<'local> {
    /// Gets the java version currently the jni environment is running on.
    fn get_java_version(&mut self) -> Result<JavaVersion>;

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
