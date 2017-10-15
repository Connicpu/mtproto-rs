use std::collections::{HashMap, BTreeMap};

use petgraph::{Graph, Direction};
use syn;

use ast::Constructor;


#[derive(Clone, Copy, Debug)]
pub(crate) struct Contains;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeckKind { Static, Dynamic }

impl TypeckKind {
    pub fn infer_basic_derives(&self) -> Vec<&'static str> {
        match *self {
            TypeckKind::Static => vec!["Clone", "Debug", "Serialize", "Deserialize", "MtProtoSized"],
            TypeckKind::Dynamic => vec!["Clone", "Debug", "Serialize", "MtProtoSized"],
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConstructorInputData<'a> {
    pub(crate) ty: Vec<Vec<syn::Ident>>,
    pub(crate) kind: TypeckKind,
    pub(crate) deps: Vec<Vec<Vec<syn::Ident>>>, // FIXME: ahhhh my eyes
    pub(crate) ctor: &'a Constructor,
}

#[derive(Clone, Debug)]
pub(crate) struct ConstructorOutputData<'a> {
    ty: Vec<Vec<syn::Ident>>,
    kind: TypeckKind,
    ctor: &'a Constructor,
}


pub(crate) fn build_transform_dag<'a>(ctors_input_data: Vec<ConstructorInputData<'a>>)
    -> Graph<ConstructorOutputData<'a>, Contains>
{
    let mut graph = Graph::new();
    let mut node_idx_ctors = HashMap::new();

    // BUILD DAG
    for input_data in ctors_input_data {
        let output_data = ConstructorOutputData {
            ty: input_data.ty.clone(),
            kind: input_data.kind,
            ctor: input_data.ctor,
        };

        let node_idx = graph.add_node(output_data);
        node_idx_ctors.insert(node_idx, input_data);
    }

    for (from_node_idx, input_data) in &node_idx_ctors {
        {
            let ConstructorOutputData { ref ty, .. } = graph[*from_node_idx];
            assert_eq!(ty, &input_data.ty);
        }

        for field_dep in &input_data.deps {
            let maybe_dep_node_idx = node_idx_ctors.iter()
                .find(|&(_, v)| {
                    field_dep.iter().any(|field_dep_component| &v.ty[0] == field_dep_component)
                })
                .map(|(k, _)| k);

            let dep_node_idx = match maybe_dep_node_idx {
                Some(idx) => idx,
                None => continue,
            };

            graph.add_edge(*from_node_idx, *dep_node_idx, Contains);
        }
    }

    // TRANSFORM DAG
    for from_node_idx in node_idx_ctors.keys() {
        let ConstructorOutputData { kind, .. } = graph[*from_node_idx];

        if kind == TypeckKind::Dynamic {
            let mut neighbors = graph.neighbors_directed(*from_node_idx, Direction::Incoming).detach();
            while let Some(neighbor_node_idx) = neighbors.next_node(&graph) {
                graph[neighbor_node_idx].kind = TypeckKind::Dynamic;
            }
        }
    }

    graph
}

pub(crate) fn analyze_dag<'a>(dag: Graph<ConstructorOutputData<'a>, Contains>)
    -> BTreeMap<&'a Constructor, TypeckKind>
{
    let (nodes, _) = dag.into_nodes_edges();
    let ctors_typeck_kinds = nodes.into_iter().map(|node| {
        let ConstructorOutputData { kind, ctor, .. } = node.weight;

        (ctor, kind)
    }).collect();

    ctors_typeck_kinds
}
