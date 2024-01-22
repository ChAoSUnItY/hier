use hier::jni_env;
use hier::HierExt;

fn main() {
    let mut env = jni_env().unwrap();
    let mut integer_class = env.lookup_class("java/lang/Integer").unwrap();
    let mut interfaces = integer_class.interfaces(&mut env).unwrap();
    let interface_names = interfaces.iter_mut()
        .map(|interface_class| interface_class.class_name(&mut env))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("{interface_names:#?}");
}
