use dot_writer::DotWriter;
use jni::{descriptors::Desc, objects::{JClass, GlobalRef}, JNIEnv};

use crate::{errors::HierResult as Result, class::{jclass_from_instance, HierExt}};

pub fn generate_class_hierarchy_tree<'local, 'other_local, T>(
    env: &mut JNIEnv<'local>,
    class: T,
) -> Result<String>
where
    T: Desc<'local, JClass<'other_local>>,
{
    let class = class.lookup(env)?;
    let mut output = vec![];

    {
        let mut writer = DotWriter::from(&mut output);
        writer.set_pretty_print(false);
        let mut scope = writer.graph();

        let classes = collect_superclasses_and_interfaces(env, class.as_ref())?;
        let class_names = classes.iter().map(|(class, derived_class)| {
            Ok((env.class_name(class)?, env.class_name(derived_class)?))
        }).collect::<Result<Vec<_>>>()?;

        println!("{:?}", class_names);

        for (to_class, from_class) in class_names {
            scope.edge(format!("\"{from_class}\""), format!("\"{to_class}\""));
        }
    }

    Ok(String::from_utf8(output).unwrap())
}

fn collect_superclasses_and_interfaces<'local, 'other_local>(env: &mut JNIEnv<'local>, class: &JClass<'other_local>) -> Result<Vec<(GlobalRef, GlobalRef)>> {
    let mut classes = vec![];
    let mut class = jclass_from_instance(env, class)?;

    loop {
        let interfaces = env.interfaces(&class)?
            .into_iter()
            .map(|interface_glob_ref| (class.clone(), interface_glob_ref))
            .collect::<Vec<_>>();

        classes.extend_from_slice(&interfaces);
        
        let superclass = env.lookup_superclass(&class)?;

        match superclass {
            Some(superclass) => {
                classes.push((class.clone(), superclass.clone()));

                class = superclass;
            }
            None => break
        }
    }

    Ok(classes)
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    use crate::{jni_env, class::HierExt, errors::HierResult as Result};

    use super::generate_class_hierarchy_tree;

    #[test]
    #[serial]
    fn test_graph() -> Result<()> {
        let mut env = jni_env()?;
        let class = env.lookup_class("java/lang/Integer")?;
        let graph = generate_class_hierarchy_tree(&mut env, &class)?;

        println!("{:}", graph);

        env.free_lookup()?;
    
        Ok(())
    }
}
