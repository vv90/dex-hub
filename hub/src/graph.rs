use std::collections::HashSet;

use petgraph::{algo::kosaraju_scc, prelude::*};

use crate::tokens::TokenId;

pub enum AdjacentTokens {
    Directed(TokenId, TokenId),
    Undirected(TokenId, TokenId),
}

pub trait TokenAdjacency<T> {
    fn adjacent_tokens(&self) -> AdjacentTokens;
    fn id(&self) -> T;
}

// since GraphMap does not allow parallel edges,
// multiple node connections are listed in the edge weight as HashSet
pub struct DexGraph<T: Eq + std::hash::Hash>(DiGraphMap<TokenId, HashSet<T>>);

impl<T: Eq + std::hash::Hash> DexGraph<T> {
    pub fn new() -> Self {
        DexGraph(DiGraphMap::new())
    }

    fn with_directed_edge(self, token0: TokenId, token1: TokenId, id: T) -> Self {
        let DexGraph(mut graph) = self;
        match graph.edge_weight_mut(token0, token1) {
            Some(ids) => {
                ids.insert(id);
            }
            None => {
                graph.add_edge(token0, token1, HashSet::from([id]));
            }
        }
        DexGraph(graph)
    }

    fn with_adjacency<A: TokenAdjacency<T>>(self, adjacency: &A) -> Self {
        match adjacency.adjacent_tokens() {
            AdjacentTokens::Directed(token0, token1) => {
                self.with_directed_edge(token0, token1, adjacency.id())
            }
            AdjacentTokens::Undirected(token0, token1) => self
                .with_directed_edge(token0, token1, adjacency.id())
                .with_directed_edge(token1, token0, adjacency.id()),
        }
    }

    pub fn with_adjacent_tokens<A: TokenAdjacency<T>>(self, adjacencies: &[A]) -> Self {
        adjacencies
            .iter()
            .fold(self, |graph, adjacency| graph.with_adjacency(adjacency))
    }

    // removes the node and returns the updated graph and neighbors of the removed node
    fn with_node_removed(self, token_id: TokenId) -> (Self, Vec<TokenId>) {
        let DexGraph(mut graph) = self;
        let neighbors = graph.neighbors(token_id).collect::<Vec<_>>();
        graph.remove_node(token_id);
        (DexGraph(graph), neighbors)
    }

    fn prune_recursive(self, token_id: TokenId, pruned_count: usize) -> (Self, usize) {
        let DexGraph(graph) = self;

        let mut incoming_iter = graph
            .edges_directed(token_id, Direction::Incoming)
            .map(|(_, _, ids)| ids);
        let mut outgoing_iter = graph
            .edges_directed(token_id, Direction::Outgoing)
            .map(|(_, _, ids)| ids);

        let node_to_remove = match (
            incoming_iter.next().map(|ids| (ids, incoming_iter.next())),
            outgoing_iter.next().map(|ids| (ids, outgoing_iter.next())),
        ) {
            // multiple incoming and multiple outgoing edges
            (
                Some((_incoming_ids, Some(_next_incoming_ids))),
                Some((_outgoing_ids, Some(_next_outgoing_ids))),
            ) => None,
            // multiple incoming, single outgoing edge
            (Some((_incoming_ids, Some(_next_incoming_ids))), Some((_outgoing_ids, None))) => None,
            // single incoming, multiple outgoing edges
            (Some((_incoming_ids, None)), Some((_outgoing_ids, Some(_next_outgoing_ids)))) => None,
            // single incoming and single outgoing edge
            (Some((incoming_ids, None)), Some((outgoing_ids, None))) => {
                // check if there's more than one unique adjacency id into/out of the node
                if incoming_ids.union(outgoing_ids).count() > 1 {
                    None
                } else {
                    // it's a dead-end node
                    Some(token_id)
                }
            }
            // it's sink node
            (Some(_), None) => Some(token_id),
            // it's source node
            (None, Some(_)) => Some(token_id),
            // it's disconnected node
            (None, None) => Some(token_id),
        };

        match node_to_remove {
            Some(token_id) => {
                // remove the node and check/prune it's neighbors recursively
                let (updated_graph, neighbors) = DexGraph(graph).with_node_removed(token_id);
                neighbors
                    .into_iter()
                    .fold((updated_graph, pruned_count + 1), |(g, s), t| {
                        g.prune_recursive(t, s)
                    })
            }
            None => (DexGraph(graph), pruned_count),
        }
    }

    // removes all sink/source nodes and dead-ends (single incoming and outgoing connection using the same adjacency id)
    pub fn pruned(self) -> Self {
        let mut current_graph = self;
        loop {
            let (pruned_graph, pruned_count) = current_graph
                .tokens()
                .into_iter()
                .fold((current_graph, 0), |(g, s), t| g.prune_recursive(t, s));

            current_graph = pruned_graph;

            if pruned_count == 0 {
                break;
            }
        }
        current_graph
    }

    pub fn components(&self) -> Vec<Vec<TokenId>> {
        let DexGraph(graph) = self;
        kosaraju_scc(graph)
    }

    pub fn tokens_count(&self) -> usize {
        let DexGraph(graph) = self;
        graph.node_count()
    }

    pub fn tokens(&self) -> HashSet<TokenId> {
        let DexGraph(graph) = self;
        graph.nodes().collect()
    }
}
