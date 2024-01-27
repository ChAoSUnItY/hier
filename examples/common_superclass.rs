use hier::class::Class;
use hier::errors::HierError;
use hier::jni_env;
use hier::HierExt;
use jni::JNIEnv;

fn main() {
    let mut env = jni_env().unwrap();
    let mut integer_class = env.lookup_class("java.lang.Integer").unwrap();
    let mut float_class = env.lookup_class("java.lang.Float").unwrap();
    let mut most_common_superclass =
        find_most_common_superclass(&mut env, &mut integer_class, &mut float_class).unwrap();

    println!("{}", most_common_superclass.name(&mut env).unwrap());
}

fn find_most_common_superclass(
    env: &mut JNIEnv,
    class1: &mut Class,
    class2: &mut Class,
) -> Result<Class, HierError> {
    if class2.is_assignable_from(env, class1)? {
        return Ok(class1.clone());
    }

    if class1.is_assignable_from(env, class2)? {
        return Ok(class2.clone());
    }

    if class1.is_interface(env)? || class2.is_interface(env)? {
        return env.lookup_class("java.lang.Object");
    }

    let mut cls1 = class1.clone();
    while {
        cls1 = match cls1.superclass(env)? {
            Some(superclass) => superclass,
            None => return Ok(cls1),
        };

        !cls1.is_assignable_from(env, class2)?
    } {}

    Ok(cls1)
}
