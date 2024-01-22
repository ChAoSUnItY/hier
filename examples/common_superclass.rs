use hier::jni_env;
use hier::HierExt;

fn main() {
    let mut env = jni_env().unwrap();
    let mut integer_class = env.lookup_class("java/lang/Integer").unwrap();
    let mut float_class = env.lookup_class("java/lang/Float").unwrap();
    let mut common_superclass = integer_class
        .common_superclass(&mut env, &mut float_class)
        .unwrap();
    let cs_class_name = common_superclass.class_name(&mut env).unwrap();

    println!("{cs_class_name}");
}
