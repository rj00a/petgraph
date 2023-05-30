#![cfg(feature = "stable_graph")]

extern crate itertools;
extern crate petgraph;
#[macro_use]
extern crate defmac;

use std::collections::HashSet;

use itertools::assert_equal;
use petgraph::{
    algo::{kosaraju_scc, min_spanning_tree, tarjan_scc},
    dot::Dot,
    prelude::*,
    stable_graph::{edge_index as e, node_index as n},
    visit::{EdgeIndexable, IntoEdgeReferences, IntoNodeReferences, NodeIndexable},
    EdgeType,
};

fn assert_sccs_eq(mut res: Vec<Vec<NodeIndex>>, normalized: Vec<Vec<NodeIndex>>) {
    // normalize the result and compare with the answer.
    for scc in &mut res {
        scc.sort();
    }
    // sort by minimum element
    res.sort_by(|v, w| v[0].cmp(&w[0]));
    assert_eq!(res, normalized);
}

fn scc_graph() -> StableGraph<(), ()> {
    let mut gr: StableGraph<(), ()> = StableGraph::from_edges(&[
        (6, 0),
        (0, 3),
        (3, 6),
        (8, 6),
        (8, 2),
        (2, 5),
        (5, 8),
        (7, 5),
        (1, 7),
        (7, 4),
        (4, 1),
    ]);
    // make an identical replacement of n(4) and leave a hole
    let x = gr.add_node(());
    gr.add_edge(n(7), x, ());
    gr.add_edge(x, n(1), ());
    gr.remove_node(n(4));
    gr
}

// TODO: move to algo
#[test]
fn test_scc() {
    let gr = scc_graph();
    println!("{:?}", gr);

    let x = n(gr.node_bound() - 1);
    assert_sccs_eq(kosaraju_scc(&gr), vec![
        vec![n(0), n(3), n(6)],
        vec![n(1), n(7), x],
        vec![n(2), n(5), n(8)],
    ]);
}

// TODO: move to algo
#[test]
fn test_tarjan_scc() {
    let gr = scc_graph();

    let x = n(gr.node_bound() - 1);
    assert_sccs_eq(tarjan_scc(&gr), vec![
        vec![n(0), n(3), n(6)],
        vec![n(1), n(7), x],
        vec![n(2), n(5), n(8)],
    ]);
}

fn make_graph<Ty>() -> StableGraph<(), i32, Ty>
where
    Ty: EdgeType,
{
    let mut gr = StableGraph::default();
    let mut c = 0..;
    let mut e = || -> i32 { c.next().unwrap() };
    gr.extend_with_edges(&[
        (6, 0, e()),
        (0, 3, e()),
        (3, 6, e()),
        (8, 6, e()),
        (8, 2, e()),
        (2, 5, e()),
        (5, 8, e()),
        (7, 5, e()),
        (1, 7, e()),
        (7, 4, e()),
        (8, 6, e()), // parallel edge
        (4, 1, e()),
    ]);
    // make an identical replacement of n(4) and leave a hole
    let x = gr.add_node(());
    gr.add_edge(n(7), x, e());
    gr.add_edge(x, n(1), e());
    gr.add_edge(x, x, e()); // make two self loops
    let rm_self_loop = gr.add_edge(x, x, e());
    gr.add_edge(x, x, e());
    gr.remove_node(n(4));
    gr.remove_node(n(6));
    gr.remove_edge(rm_self_loop);
    gr
}

defmac!(edges ref gr, x => gr.edges(x).map(|r| (r.target(), *r.weight())));

// TODO: test directed
#[test]
fn test_edges_directed() {
    let gr = make_graph::<Directed>();
    let x = n(9);
    assert_equal(edges!(&gr, x), vec![(x, 16), (x, 14), (n(1), 13)]);
    assert_equal(edges!(&gr, n(0)), vec![(n(3), 1)]);
    assert_equal(edges!(&gr, n(4)), vec![]);
}

// TODO: test directed
#[test]
fn test_edge_references() {
    let gr = make_graph::<Directed>();
    assert_eq!(gr.edge_count(), gr.edge_references().count());
}

// TODO
#[test]
fn test_edge_iterators_directed() {
    let gr = make_graph::<Directed>();
    for i in gr.node_indices() {
        itertools::assert_equal(gr.edges_directed(i, Outgoing), gr.edges(i));
        for edge in gr.edges_directed(i, Outgoing) {
            assert_eq!(
                edge.source(),
                i,
                "outgoing edges should have a fixed source"
            );
        }
    }
    let mut incoming = vec![Vec::new(); gr.node_bound()];

    for i in gr.node_indices() {
        for j in gr.neighbors(i) {
            incoming[j.index()].push(i);
        }
    }

    println!("{:#?}", gr);
    for i in gr.node_indices() {
        itertools::assert_equal(
            gr.edges_directed(i, Incoming).map(|e| e.source()),
            incoming[i.index()].iter().rev().cloned(),
        );
        for edge in gr.edges_directed(i, Incoming) {
            assert_eq!(
                edge.target(),
                i,
                "incoming edges should have a fixed target"
            );
        }
    }
}

// TODO
#[test]
fn test_node_references() {
    let gr = scc_graph();

    itertools::assert_equal(gr.node_references().map(|(i, _)| i), gr.node_indices());
}

// TODO
#[test]
fn iterators_undir() {
    let mut g = StableUnGraph::<_, _>::default();
    let a = g.add_node(0);
    let b = g.add_node(1);
    let c = g.add_node(2);
    let d = g.add_node(3);
    g.extend_with_edges(&[(a, b, 1), (a, c, 2), (b, c, 3), (c, c, 4), (a, d, 5)]);
    g.remove_node(b);

    itertools::assert_equal(g.neighbors(a), vec![d, c]);
    itertools::assert_equal(g.neighbors(c), vec![c, a]);
    itertools::assert_equal(g.neighbors(d), vec![a]);

    // the node that was removed
    itertools::assert_equal(g.neighbors(b), vec![]);

    // remove one more
    g.remove_node(c);
    itertools::assert_equal(g.neighbors(c), vec![]);
}

// TODO: directed
#[test]
fn iter_multi_edges() {
    let mut gr = StableGraph::new();
    let a = gr.add_node("a");
    let b = gr.add_node("b");
    let c = gr.add_node("c");

    let mut connecting_edges = HashSet::new();

    gr.add_edge(a, a, ());
    connecting_edges.insert(gr.add_edge(a, b, ()));
    gr.add_edge(a, c, ());
    gr.add_edge(c, b, ());
    connecting_edges.insert(gr.add_edge(a, b, ()));
    gr.add_edge(b, a, ());

    let mut iter = gr.edges_connecting(a, b);

    let edge_id = iter.next().unwrap().id();
    assert!(connecting_edges.contains(&edge_id));
    connecting_edges.remove(&edge_id);

    let edge_id = iter.next().unwrap().id();
    assert!(connecting_edges.contains(&edge_id));
    connecting_edges.remove(&edge_id);

    assert_eq!(None, iter.next());
    assert!(connecting_edges.is_empty());
}

// TODO: move to different module; use insta
#[test]
fn dot() {
    let mut gr = StableGraph::new();
    let a = gr.add_node("x");
    let b = gr.add_node("y");
    gr.add_edge(a, a, "10");
    gr.add_edge(a, b, "20");
    let dot_output = format!("{}", Dot::new(&gr));
    assert_eq!(
        dot_output,
        r#"digraph {
    0 [ label = "x" ]
    1 [ label = "y" ]
    0 -> 0 [ label = "10" ]
    0 -> 1 [ label = "20" ]
}
"#
    );
}

defmac!(iter_eq a, b => a.eq(b));
defmac!(nodes_eq ref a, ref b => a.node_references().eq(b.node_references()));
defmac!(edgew_eq ref a, ref b => a.edge_references().eq(b.edge_references()));
defmac!(edges_eq ref a, ref b =>
        iter_eq!(
            a.edge_references().map(|e| (e.source(), e.target())),
            b.edge_references().map(|e| (e.source(), e.target()))));

use petgraph::{data::FromElements, stable_graph::StableGraph};

// TODO: move to algo
#[test]
fn from_min_spanning_tree() {
    let mut g = StableGraph::new();
    let mut nodes = Vec::new();
    for _ in 0..6 {
        nodes.push(g.add_node(()));
    }
    let es = [(4, 5), (3, 4), (3, 5)];
    for &(a, b) in es.iter() {
        g.add_edge(NodeIndex::new(a), NodeIndex::new(b), ());
    }
    for i in 0..3 {
        let _ = g.remove_node(nodes[i]);
    }
    let _ = StableGraph::<(), (), Undirected, usize>::from_elements(min_spanning_tree(&g));
}

// TODO
#[test]
fn weights_mut_iterator() {
    let mut gr = StableGraph::new();
    let a = gr.add_node(1);
    let b = gr.add_node(2);
    let c = gr.add_node(3);
    let e1 = gr.add_edge(a, a, 10);
    let e2 = gr.add_edge(a, b, 20);
    let e3 = gr.add_edge(b, c, 30);
    let e4 = gr.add_edge(a, c, 40);

    for n in gr.node_weights_mut() {
        *n += 1;
    }
    assert_eq!(gr[a], 2);
    assert_eq!(gr[b], 3);
    assert_eq!(gr[c], 4);

    for e in gr.edge_weights_mut() {
        *e -= 1;
    }
    assert_eq!(gr[e1], 9);
    assert_eq!(gr[e2], 19);
    assert_eq!(gr[e3], 29);
    assert_eq!(gr[e4], 39);

    // test on deletion
    gr.remove_node(b);
    assert_eq!(gr.node_weights_mut().count(), gr.node_count());
    assert_eq!(gr.edge_weights_mut().count(), gr.edge_count());
}
