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
