use alloy::{
    primitives::Address,
    providers::{Provider, fillers::RecommendedFillers},
};
use anyhow::Result;

use crate::{
    Blockchain,
    blockchain::{BlockNumber, BlockchainNetwork},
    chainlink::{
        get_configured_tokens_call_data::GetConfiguredTokensCallData,
        get_pools_call_data::GetPoolsCallData, pool::PoolAddress,
        remote_tokens_multicall_data::RemoteTokensMulticallData,
    },
    evm_network,
    rpc::{self, client::RpcClient},
    tokens::TokenAddress,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bridge {
    address: Address,
    local_token_address: Address,
    blockchain: Blockchain,
    remote_token: TokenAddress,
}

impl Bridge {
    pub fn pool_address(&self) -> PoolAddress {
        PoolAddress(self.address, self.blockchain)
    }

    pub fn local_token(&self) -> TokenAddress {
        TokenAddress(self.local_token_address, self.blockchain)
    }

    pub fn remote_token(&self) -> TokenAddress {
        self.remote_token
    }
}

async fn get_bridges_recursive<B: BlockchainNetwork, P: Provider<B>>(
    rpc_client: &RpcClient<B, P>,
    call_data: GetConfiguredTokensCallData<B>,
    blockchains: &[Blockchain],
    block_number: BlockNumber<B>,
    mut bridges: Vec<Bridge>,
) -> Result<Vec<Bridge>> {
    let new_tokens = rpc_client.call(call_data.clone()).await?;
    let loaded_items_count = new_tokens.len();
    let get_pools_call_data = GetPoolsCallData::<B>::create(&new_tokens)?;
    let new_token_pools = rpc_client.call(get_pools_call_data).await?;
    let remote_blockchains = blockchains
        .into_iter()
        .copied()
        .filter(|&bc| bc != B::BLOCKCHAIN)
        .collect::<Vec<_>>();
    let remote_tokens_calls_data = new_token_pools
        .iter()
        .filter_map(|p| p.as_ref())
        .flat_map(|p| {
            remote_blockchains
                .iter()
                .map(|&bc| RemoteTokensMulticallData::new(*p, bc))
        })
        .collect::<Vec<_>>();
    let new_remote_tokens: Vec<Option<TokenAddress>> = rpc_client
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
                .flat_map(|(t, p)| remote_blockchains.iter().copied().map(move |bc| (t, p, bc))),
        )
        .filter_map(
            |(
                maybe_remote_token,
                (
                    TokenAddress(local_token_address, local_blockchain),
                    PoolAddress(pool_address, pool_blockchain),
                    remote_blockchain,
                ),
            )| {
                maybe_remote_token.map(|remote_token| {
                    assert_eq!(remote_token.blockchain(), remote_blockchain);
                    assert_eq!(local_blockchain, pool_blockchain);

                    Bridge {
                        address: pool_address,
                        local_token_address,
                        blockchain: local_blockchain,
                        remote_token,
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
            blockchains,
            block_number,
            bridges,
        ))
        .await
    }
}

pub async fn get_bridges_for_blockchain<B: BlockchainNetwork + RecommendedFillers>(
    // blockchain: Blockchain,
    blockchains: &[Blockchain],
) -> Result<Vec<Bridge>> {
    let rpc_client = rpc::client::init_client::<B>().await?;
    let block_number = rpc_client.get_block_number().await?;

    get_bridges_recursive(
        &rpc_client,
        GetConfiguredTokensCallData::new(0, 1000),
        blockchains,
        block_number,
        Vec::new(),
    )
    .await
}

pub async fn get_bridges(blockchains: &[Blockchain]) -> Result<Vec<Bridge>> {
    let mut aggregated_bridges = Vec::new();

    for blockchain in blockchains {
        let blockchain_bridges = match blockchain {
            Blockchain::Ethereum => {
                get_bridges_for_blockchain::<evm_network::Ethereum>(blockchains).await?
            }
            Blockchain::BSC => get_bridges_for_blockchain::<evm_network::BSC>(blockchains).await?,
            Blockchain::Arbitrum => {
                get_bridges_for_blockchain::<evm_network::Arbitrum>(blockchains).await?
            }
        };
        aggregated_bridges.extend(blockchain_bridges);
    }

    Ok(aggregated_bridges)
}
