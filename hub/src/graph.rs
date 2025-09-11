use anyhow::Result;
use petgraph::{
    dot::{Config, Dot},
    prelude::*,
};
use std::{collections::HashMap, fs::File, io::Write};

use crate::tokens::{self, TokenId};

pub trait TokenAdjacency<T: std::fmt::Debug> {
    fn adjacent_tokens(&self) -> [TokenId; 2];
    fn pool_id(&self) -> T;
}

pub struct TokensGraph<T: std::fmt::Debug> {
    graph: StableGraph<TokenId, T, Undirected>,
    node_indexes: HashMap<TokenId, NodeIndex>,
}

impl<T: std::fmt::Debug> TokensGraph<T> {
    pub fn new() -> Self {
        Self {
            graph: StableGraph::default(),
            node_indexes: HashMap::new(),
        }
    }

    fn with_token(mut self, token: TokenId) -> (Self, NodeIndex) {
        let index = *self
            .node_indexes
            .entry(token)
            .or_insert_with(|| self.graph.add_node(token));

        (self, index)
    }

    fn with_pool<A: TokenAdjacency<T>>(self, pool: &A) -> Self {
        let [token0, token1] = pool.adjacent_tokens();

        let (tokens_graph, token0_index) = self.with_token(token0);
        let (mut tokens_graph, token1_index) = tokens_graph.with_token(token1);

        tokens_graph
            .graph
            .add_edge(token0_index, token1_index, pool.pool_id());

        tokens_graph
    }

    pub fn with_pools<A: TokenAdjacency<T>>(self, pools: &[A]) -> Self {
        pools
            .iter()
            .filter(|pool| {
                let [token0, token1] = pool.adjacent_tokens();
                !tokens::BLACKLIST.contains(&token0) && !tokens::BLACKLIST.contains(&token1)
            })
            .fold(self, |graph, pool| graph.with_pool(pool))
    }

    fn with_node_removed(mut self, node_index: NodeIndex) -> Self {
        if let Some(weight) = self.graph.remove_node(node_index) {
            self.node_indexes.remove(&weight);
        }
        self
    }

    fn node_recursive_check_and_remove(self, node_index: NodeIndex) -> Self {
        // let edges = self.graph.edges(node_index).next();
        let mut neighbors = self.graph.neighbors(node_index);

        match neighbors
            .next()
            .map(|first_neighbor| (first_neighbor, neighbors.next()))
        {
            Some((_, Some(_))) => {
                // has at least two neighbors
                // leave as is
                self
            }
            Some((first_neighbor, None)) => {
                // has only one neighbor
                // remove the node and recursively check the neighbor
                self.with_node_removed(node_index)
                    .node_recursive_check_and_remove(first_neighbor)
            }
            None => {
                // remove node
                self.with_node_removed(node_index)
            }
        }
    }

    pub fn with_dead_end_tokens_removed(self) -> Self {
        let node_indexes = self.node_indexes.values().copied().collect::<Vec<_>>();
        node_indexes.into_iter().fold(self, |graph, node_index| {
            graph.node_recursive_check_and_remove(node_index)
        })
    }

    pub fn tokens_count(&self) -> usize {
        self.node_indexes.len()
    }

    pub fn contains_token(&self, token_id: &TokenId) -> bool {
        self.node_indexes.contains_key(token_id)
    }
}

pub fn render_tokens_graph<T: std::fmt::Debug, Ty: petgraph::EdgeType>(
    graph: &Graph<TokenId, T, Ty>,
    file_path: &str,
    token_label_fn: &dyn Fn(&TokenId) -> String,
    pool_label_fn: &dyn Fn(&T) -> String,
) -> Result<()> {
    let edge_attr =
        |_: &Graph<TokenId, T, Ty>, e: petgraph::graph::EdgeReference<'_, T>| -> String {
            let pool_id: &T = e.weight();

            format!("label = \"{}\" ", pool_label_fn(pool_id))
        };

    let node_attr = |_: &Graph<TokenId, T, Ty>,
                     (_, token_id): (petgraph::graph::NodeIndex, &TokenId)|
     -> String { format!("label = \"{}\" ", token_label_fn(token_id)) };

    // let graph_clone = self.graph.clone();
    let dot = Dot::with_attr_getters(
        graph,
        &[Config::NodeNoLabel, Config::EdgeNoLabel],
        &edge_attr,
        &node_attr,
    );

    let dot_string = format!("{:?}", dot);
    let mut file = File::create(file_path)?;
    file.write_all(dot_string.as_bytes())?;

    Ok(())
}
