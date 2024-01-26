# Hier

Hier is a library supports JVM class hierarchy lookup by extending
JNI interface.

## Installation

To use this Hier, you'll need to have complete installation of JDK 
on machine (additionally, environment variables should be set, 
e.g. `JAVA_HOME`).

## Usage

### Get common super class of 2 classes

```rs
use hier::jni_env;
use hier::HierExt;

fn main() {
    let mut env = jni_env().unwrap();
    let mut integer_class = env.lookup_class("java.lang.Integer").unwrap();
    let mut float_class = env.lookup_class("java.lang.Float").unwrap();
    let mut common_superclass = integer_class.common_superclass(&mut env, &mut float_class).unwrap();
    let cs_class_name = common_superclass.class_name(&mut env).unwrap();

    println!("{cs_class_name}");
}
```

### Get derived interface of class

```rs
use hier::jni_env;
use hier::HierExt;

fn main() {
    let mut env = jni_env().unwrap();
    let mut integer_class = env.lookup_class("java.lang.Integer").unwrap();
    let mut interfaces = integer_class.interfaces(&mut env).unwrap();
    let interface_names = interfaces.iter_mut()
        .map(|interface_class| interface_class.class_name(&mut env))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("{interface_names:#?}");
}
```

## Performance

Finding class through JNI costs huge amount of a performance. Hier caches all found classes, 
to use cached classes, use `HierExt::lookup_class`. To release the cache, use `HierExt::free_lookup`.

Additionally, `HierExt::common_superclass` always uses cached class to find.

## License
Hier is licensed under MIT License.
