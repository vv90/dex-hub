use alloy::{
    primitives::Address,
    providers::{Provider, fillers::RecommendedFillers},
};
use anyhow::Result;

use crate::{
    Blockchain,
    blockchain::{BlockNumber, BlockchainNetwork},
    chainlink::{
        chain_selector::{ChainSelector, chain_selector},
        get_configured_tokens_call_data::GetConfiguredTokensCallData,
        get_pools_call_data::GetPoolsCallData,
        pool::PoolAddress,
        remote_tokens_multicall_data::RemoteTokensMulticallData,
    },
    evm_network,
    rpc::{self, client::RpcClient},
    tokens::TokenAddress,
};

#[derive(Debug, Clone, Copy)]
pub struct BridgeSource {
    address: Address,
    local_token_address: Address,
    local_blockchain: Blockchain,
}

impl BridgeSource {
    pub fn local_token(&self) -> TokenAddress {
        TokenAddress(self.local_token_address, self.local_blockchain)
    }

    pub fn bridge_address(&self) -> PoolAddress {
        PoolAddress(self.address, self.local_blockchain)
    }
}

#[derive(Debug, Clone)]
pub struct BridgeTarget {
    pub chain_selector: ChainSelector,
    pub remote_token: bytes::Bytes,
}

#[derive(Debug, Clone)]
pub struct Bridge {
    pub source: BridgeSource,
    pub target: BridgeTarget,
}

impl Bridge {
    pub fn pool_address(&self) -> PoolAddress {
        PoolAddress(self.source.address, self.source.local_blockchain)
    }

    pub fn local_token(&self) -> TokenAddress {
        TokenAddress(
            self.source.local_token_address,
            self.source.local_blockchain,
        )
    }

    // pub fn remote_token(&self) -> (ChainSelector, bytes::Bytes) {
    //     self.target.remote_token.clone()
    // }
}

async fn get_bridges_recursive<B: BlockchainNetwork, P: Provider<B>>(
    rpc_client: &RpcClient<B, P>,
    call_data: GetConfiguredTokensCallData<B>,
    // blockchains: &[Blockchain],
    chain_selectors: &[ChainSelector],
    block_number: BlockNumber<B>,
    mut bridges: Vec<Bridge>,
) -> Result<Vec<Bridge>> {
    let new_tokens = rpc_client.call(call_data.clone()).await?;
    let loaded_items_count = new_tokens.len();
    let get_pools_call_data = GetPoolsCallData::<B>::create(&new_tokens)?;
    let new_token_pools = rpc_client.call(get_pools_call_data).await?;

    let remote_chain_selectors = chain_selectors
        .into_iter()
        .copied()
        .filter(|&cs| cs != chain_selector(B::BLOCKCHAIN))
        .collect::<Vec<_>>();

    let remote_tokens_calls_data = new_token_pools
        .iter()
        .filter_map(|p| p.as_ref())
        .flat_map(|p| {
            remote_chain_selectors
                .iter()
                .map(|&cs| RemoteTokensMulticallData::new(*p, cs))
        })
        .collect::<Vec<_>>();
    let new_remote_tokens = rpc_client
        .get_multicall(&remote_tokens_calls_data, block_number)
        .await?
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let new_bridges = new_remote_tokens
        .into_iter()
        .zip(
            new_tokens
                .into_iter()
                .zip(new_token_pools.into_iter())
                .filter_map(|(t, op)| op.map(|p| (t, p)))
                .flat_map(|(t, p)| {
                    remote_chain_selectors
                        .iter()
                        .copied()
                        .map(move |cs| (t, p, cs))
                }),
        )
        .filter_map(
            |(
                maybe_remote_token,
                (
                    TokenAddress(local_token_address, local_blockchain),
                    PoolAddress(pool_address, pool_blockchain),
                    remote_chain_selector,
                ),
            )| {
                maybe_remote_token.map(|(remote_token_chain_selector, remote_token_bytes)| {
                    assert_eq!(remote_token_chain_selector, remote_chain_selector);
                    assert_eq!(local_blockchain, pool_blockchain);

                    Bridge {
                        source: BridgeSource {
                            address: pool_address,
                            local_token_address,
                            local_blockchain,
                        },
                        target: BridgeTarget {
                            chain_selector: remote_token_chain_selector,
                            remote_token: remote_token_bytes,
                        },
                    }
                })
            },
        );

    bridges.extend(new_bridges);

    if loaded_items_count < call_data.max_count() as usize {
        Ok(bridges)
    } else {
        Box::pin(get_bridges_recursive(
            rpc_client,
            GetConfiguredTokensCallData::next(&call_data),
            chain_selectors,
            block_number,
            bridges,
        ))
        .await
    }
}

pub async fn get_bridges_for_blockchain<B: BlockchainNetwork + RecommendedFillers>(
    // blockchain: Blockchain,
    // blockchains: &[Blockchain],
    chain_selectors: &[ChainSelector],
) -> Result<Vec<Bridge>> {
    let rpc_client = rpc::client::init_client::<B>().await?;
    let block_number = rpc_client.get_block_number().await?;

    get_bridges_recursive(
        &rpc_client,
        GetConfiguredTokensCallData::new(0, 1000),
        chain_selectors,
        block_number,
        Vec::new(),
    )
    .await
}

pub async fn get_bridges(
    local_blockchains: &[Blockchain],
    remote_chain_selectors: &[ChainSelector],
) -> Result<Vec<Bridge>> {
    let mut aggregated_bridges = Vec::new();

    for blockchain in local_blockchains {
        let blockchain_bridges = match blockchain {
            Blockchain::Ethereum => {
                get_bridges_for_blockchain::<evm_network::Ethereum>(remote_chain_selectors).await?
            }
            Blockchain::BSC => {
                get_bridges_for_blockchain::<evm_network::BSC>(remote_chain_selectors).await?
            }
            Blockchain::Arbitrum => {
                get_bridges_for_blockchain::<evm_network::Arbitrum>(remote_chain_selectors).await?
            }
        };
        aggregated_bridges.extend(blockchain_bridges);
    }

    Ok(aggregated_bridges)
}
