use dot_writer::DotWriter;
use jni::{
    descriptors::Desc,
    objects::{GlobalRef, JClass},
    JNIEnv,
};

use crate::{
    class::{jclass_from_instance, HierExt},
    errors::HierResult as Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeType {
    Class,
    Interface,
}

#[derive(Debug, Clone)]
struct Node {
    inner: GlobalRef,
    node_type: NodeType,
}

impl Node {
    pub fn new<'local>(glob_ref: GlobalRef, node_type: NodeType) -> Self {
        Self {
            inner: glob_ref,
            node_type,
        }
    }
}

#[derive(Debug, Clone)]
struct Edge {
    from: Node,
    to: Node,
}

impl Edge {
    pub fn new(from: Node, to: Node) -> Self {
        Self { from, to }
    }
}

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

        let edges = collect_edges(env, class.as_ref())?;

        for Edge { from, to } in edges {
            let from_class_name = env.class_name(&from.inner)?;
            let to_class_name = env.class_name(&to.inner)?;

            scope.edge(
                format!("\"{from_class_name}\""),
                format!("\"{to_class_name}\""),
            );
        }
    }

    Ok(String::from_utf8(output).unwrap())
}

fn collect_edges<'local, 'other_local>(
    env: &mut JNIEnv<'local>,
    class: &JClass<'other_local>,
) -> Result<Vec<Edge>> {
    let mut classes = vec![];
    let mut class = jclass_from_instance(env, class)?;

    loop {
        let class_node = Node::new(class.clone(), NodeType::Class);
        let interfaces = env
            .interfaces(&class)?
            .into_iter()
            .map(|interface_glob_ref| {
                let interface_node = Node::new(interface_glob_ref, NodeType::Interface);

                Edge::new(class_node.clone(), interface_node)
            })
            .collect::<Vec<_>>();

        classes.extend_from_slice(&interfaces);

        let superclass = env.lookup_superclass(&class)?;

        match superclass {
            Some(superclass) => {
                let superclass_node = Node::new(superclass.clone(), NodeType::Class);

                classes.push(Edge::new(class_node, superclass_node));

                class = superclass;
            }
            None => break,
        }
    }

    Ok(classes)
}

#[cfg(test)]
mod test {
    use serial_test::serial;

    use crate::{class::HierExt, errors::HierResult as Result, jni_env};

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
