use alloc::{sync::Arc, vec::Vec};
use core::{
    cmp::Ordering,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use petgraph_core::{
    data::{Build, Create},
    visit::Data,
};
use proptest::{
    arbitrary::{any, Arbitrary},
    collection::{btree_set, vec},
    sample::SizeRange,
    strategy::{Just, Strategy, TupleUnion},
};

#[derive(Debug)]
struct Edge<T, U> {
    start: T,
    end: T,
    weight: U,
}

impl<T, U> PartialEq for Edge<T, U>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        (&self.start, &self.end).eq(&(&other.start, &other.end))
    }
}

impl<T, U> Eq for Edge<T, U> where T: Eq {}

impl<T, U> PartialOrd for Edge<T, U>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (&self.start, &self.end).partial_cmp(&(&other.start, &other.end))
    }
}

impl<T, U> Ord for Edge<T, U>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.start, &self.end).cmp(&(&other.start, &other.end))
    }
}

impl<T, U> Hash for Edge<T, U>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.start, &self.end).hash(state)
    }
}

fn edge_strategy<T, U, S1, S2>(endpoints: S1, weight: Arc<S2>) -> impl Strategy<Value = Edge<T, U>>
where
    T: PartialOrd + Hash + Clone + Debug,
    U: Debug,
    S1: Strategy<Value = (T, T)>,
    S2: Strategy<Value = U>,
{
    endpoints
        .prop_flat_map(move |(start, end)| (Just(start), Just(end), Arc::clone(&weight)))
        .prop_map(move |(start, end, weight)| Edge { start, end, weight })
}

// TODO: test
pub fn graph_strategy<G>(
    self_loops: bool,
    parallel_edges: bool,
    node_range: impl Into<SizeRange>,
    edge_range: Option<Arc<dyn Fn(usize) -> SizeRange>>,
) -> impl Strategy<Value = G>
where
    G: Create + Build + Data + Debug,
    G::NodeWeight: Arbitrary + Clone + Debug,
    G::EdgeWeight: Arbitrary + Clone + Debug,
{
    let edge_range = edge_range.unwrap_or_else(|| Arc::new(|max| SizeRange::from(0..max.pow(2))));

    vec(any::<G::NodeWeight>(), node_range.into())
        .prop_flat_map(move |nodes: Vec<G::NodeWeight>| {
            let nodes_len = nodes.len();

            // generate an edge where no self edges are allowed, this allows the use in
            // a lot more graphs
            let edge_endpoints = Arc::new((0..nodes_len).prop_flat_map(move |start| {
                // if we allow self loops we simply include the start in the range for end
                let range_start = if self_loops { start } else { start + 1 };

                // start < end, this has the benefit that an undirected graph won't have any
                // parallel edges
                (range_start..nodes_len).prop_map(move |end| (start, end))
            }));

            let edge_endpoint = Arc::new(edge_strategy(
                Arc::clone(&edge_endpoints),
                Arc::new(any::<G::EdgeWeight>()),
            ));

            let edges_range = (edge_range)(nodes_len);

            // using btree_set here, as while it is slower, it is usable in no-std
            let edge_endpoints_no_parallel_edges = Arc::new(
                btree_set(Arc::clone(&edge_endpoint), edges_range.clone())
                    .prop_map(|values| values.into_iter().collect::<Vec<_>>()),
            );

            let edge_endpoints_parallel_edges = Arc::new(vec(edge_endpoint, edges_range));

            let edge_endpoints = TupleUnion::new((
                (parallel_edges.into(), edge_endpoints_parallel_edges),
                ((!parallel_edges).into(), edge_endpoints_no_parallel_edges),
            ));

            (Just(nodes), edge_endpoints)
        })
        .prop_map(move |(nodes, edges)| {
            let mut graph = G::with_capacity(nodes.len(), edges.len());

            // reference table for edges
            let nodes: Vec<_> = nodes
                .into_iter()
                .map(|weight| graph.add_node(weight))
                .collect();

            for Edge { start, end, weight } in edges {
                graph.add_edge(nodes[start], nodes[end], weight);
            }

            graph
        })
}