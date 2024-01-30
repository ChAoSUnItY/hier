# Hier

![crates.io](https://img.shields.io/crates/v/hier.svg)
[![Docs badge]][docs.rs]

Hier is a library supports JVM class hierarchy lookup by extending
JNI interface.

## Installation

To use this Hier, you'll need to have complete installation of JDK 
on machine (additionally, environment variables should be set, 
e.g. `JAVA_HOME`).

## Usage

To use without calling from an exist Java application, you'll need 
to enable `invocation` feature, then use `ClassPool::from_permanent_env`
to construct a `ClassPool`.

### Get common super class of 2 classes

```rs
use hier::class::Class;
use hier::classpool::ClassPool;
use hier::errors::HierError;

fn main() {
    let mut cp = ClassPool::from_permanent_env().unwrap();
    let mut integer_class = cp.lookup_class("java.lang.Integer").unwrap();
    let mut float_class = cp.lookup_class("java.lang.Float").unwrap();
    let mut most_common_superclass =
        find_most_common_superclass(&mut cp, &mut integer_class, &mut float_class).unwrap();

    println!("{}", most_common_superclass.name(&mut cp).unwrap());
}

fn find_most_common_superclass(
    cp: &mut ClassPool,
    class1: &mut Class,
    class2: &mut Class,
) -> Result<Class, HierError> {
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
```

### Get derived interface of class

```rs
use hier::classpool::ClassPool;

fn main() {
    let mut cp = ClassPool::from_permanent_env().unwrap();
    let mut integer_class = cp.lookup_class("java.lang.Integer").unwrap();
    let mut interfaces = integer_class.interfaces(&mut cp).unwrap();
    let interface_names = interfaces
        .iter_mut()
        .map(|interface_class| interface_class.name(&mut cp))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    println!("{interface_names:#?}");
}
```

## License
Hier is licensed under MIT License.

[Docs badge]: https://img.shields.io/badge/docs.rs-rustdoc-green
[docs.rs]: https://docs.rs/hier/
