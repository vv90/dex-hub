use std::{collections::HashSet, marker::PhantomData};

use alloy::{
    network::{Ethereum, NetworkWallet, ReceiptResponse, TransactionBuilder},
    primitives::{Address, FixedBytes, U256},
    providers::{
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
        fillers::{FillProvider, JoinFill, RecommendedFillers},
    },
    rpc::types::{Filter, Log},
    sol_types::SolCall,
};
use anyhow::{Result, anyhow};
use futures_util::StreamExt;
use reqwest::Url;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    blockchain::{BlockNumber, BlockchainNetwork},
    multicall,
    rpc::{call_data::CallData, multicall_data::MulticallData, transaction_data::TransactionData},
};

// #[cfg(feature = "testnet")]
// const ETHEREUM: &str = "sepolia";
// #[cfg(not(feature = "testnet"))]
// const ETHEREUM: &str = "ethereum";

pub struct RpcClient<B: BlockchainNetwork, T: Provider<B>> {
    provider: T,
    _blockchain_marker: PhantomData<fn() -> B>,
}

pub async fn subscribe_blocks<B: BlockchainNetwork>(
    sender: mpsc::Sender<u64>,
) -> Result<JoinHandle<Result<()>>> {
    let api_key = env!("DRPC_API_KEY");
    let ws_url = env!("DRPC_WS_URL");

    let handle = tokio::spawn(async move {
        println!("Create provider");

        let ws_connect = WsConnect::new(format!(
            "{}?network={}&dkey={}",
            ws_url,
            B::BLOCKCHAIN.name(),
            api_key
        ));
        let ws_provider = ProviderBuilder::new().connect_ws(ws_connect).await?;

        println!("Connected to provider");

        let sub = ws_provider.subscribe_blocks().await?;

        println!("Subscribed to blocks");

        let mut stream = sub.into_stream();
        while let Some(header) = stream.next().await {
            sender.send(header.number).await?;
        }

        Ok(())
    });

    Ok(handle)
}

pub async fn subscribe_topics<
    T: Sync + Send + 'static,
    B: BlockchainNetwork + RecommendedFillers,
>(
    sender: mpsc::UnboundedSender<T>,
    topics: HashSet<FixedBytes<32>>,
    filter_map_log: impl Fn(Log) -> Option<T> + Send + 'static,
) -> Result<JoinHandle<Result<()>>> {
    let api_key = env!("DRPC_API_KEY");
    let ws_url = env!("DRPC_WS_URL");

    let handle = tokio::spawn(async move {
        println!("Connecting");
        let ws_connect = WsConnect::new(format!(
            "{}?network={}&dkey={}",
            ws_url,
            B::BLOCKCHAIN.name(),
            api_key
        ));
        let ws_provider = ProviderBuilder::new_with_network::<B>()
            .connect_ws(ws_connect)
            .await?;

        println!("Connected to provider");

        // let filters = topics
        //     .into_iter()
        //     .map(|topic| Filter::new().event_signature(topic))
        //     .collect::<Vec<_>>();

        let filter = Filter::new().event_signature(topics.into_iter().collect::<Vec<_>>());

        // let subscriptions = futures_util::future::try_join_all(
        //     filters
        //         .into_iter()
        //         .map(|filter| ws_provider.subscribe_logs(&filter).into_future()),
        // )
        // .await?;
        let mut subscription = ws_provider.subscribe_logs(&filter).await?;

        println!("Subscribed to topics");

        // let mut stream = futures_util::stream::select_all(
        //     subscriptions.into_iter().map(|sub| sub.into_stream()),
        // );

        while let Ok(log) = subscription.recv().await {
            if let Some(value) = filter_map_log(log) {
                sender.send(value)?;
            }
        }

        println!("Connection closed");

        Ok(())
    });

    Ok(handle)
}

pub type NetworkProvider<B> = FillProvider<
    JoinFill<Identity, <B as RecommendedFillers>::RecommendedFillers>,
    RootProvider<B>,
    B,
>;

pub async fn init_client<B>() -> Result<RpcClient<B, NetworkProvider<B>>>
where
    B: BlockchainNetwork + RecommendedFillers,
{
    let api_key = env!("DRPC_API_KEY");
    let api_url = env!("DRPC_HTTPS_URL");

    let url = Url::parse(&format!(
        "{}?network={}&dkey={}",
        api_url,
        B::BLOCKCHAIN.name(),
        api_key
    ))?;

    let provider = ProviderBuilder::new_with_network::<B>().connect_http(url);

    Ok(RpcClient {
        provider,
        _blockchain_marker: PhantomData,
    })
}

pub async fn init_client_with_signer<B, W>(wallet: W) -> Result<RpcClient<B, impl Provider<B>>>
where
    B: BlockchainNetwork + RecommendedFillers,
    W: NetworkWallet<B> + Clone,
{
    let api_key = env!("DRPC_API_KEY");
    let api_url = env!("DRPC_HTTPS_URL");

    let url = Url::parse(&format!(
        "{}?network={}&dkey={}",
        api_url,
        B::BLOCKCHAIN.name(),
        api_key
    ))?;
    let provider = ProviderBuilder::new_with_network::<B>()
        .wallet(wallet)
        .connect_http(url);

    Ok(RpcClient {
        provider,
        _blockchain_marker: PhantomData,
    })
}

impl<B: BlockchainNetwork, T: Provider<B>> RpcClient<B, T> {
    pub async fn get_block_number(&self) -> Result<BlockNumber<B>> {
        let block_number = self.provider.get_block_number().await?;
        Ok(BlockNumber::new(block_number))
    }

    pub async fn get_balance(&self, address: Address) -> Result<U256> {
        let balance = self.provider.get_balance(address).await?;
        Ok(balance)
    }

    pub async fn get_multicall<D: MulticallData<B>>(
        &self,
        calls_data: &[D],
        block_number: BlockNumber<B>,
    ) -> Result<Vec<Result<D::Output>>> {
        let calls = calls_data
            .iter()
            .flat_map(|data| data.to_calls())
            .collect::<Vec<_>>();

        let multicall = multicall::Multicall3::aggregateCall { calls };

        let multicall_data = multicall.abi_encode();

        let tx = B::TransactionRequest::default()
            .with_to(multicall::multicall3_address(B::BLOCKCHAIN))
            .with_input(multicall_data);

        let response = self
            .provider
            .call(tx)
            .block(alloy::eips::BlockId::Number(
                alloy::eips::BlockNumberOrTag::Number(block_number.value()),
            ))
            .await?;

        let decoded_response = multicall::Multicall3::aggregateCall::abi_decode_returns(&response)?;

        let (_, results) = calls_data.into_iter().fold(
            (0, Vec::<Result<D::Output>>::new()),
            |(position, mut collection), data| -> (usize, Vec<Result<D::Output>>) {
                let next_position = position + data.size();
                let item =
                    data.decode_output(&decoded_response.returnData[position..next_position]);
                collection.push(item);
                (next_position, collection)
            },
        );

        Ok(results)
    }

    pub async fn call<D: CallData<B>>(&self, call_data: D) -> Result<D::Output> {
        let call_data_bytes = call_data.into_call_data();
        let tx = B::TransactionRequest::default()
            .with_to(call_data.contract_address())
            .with_input(call_data_bytes);

        let response = self.provider.call(tx).await?;

        let decoded_output = call_data.decode_call_output(response)?;

        Ok(decoded_output)
    }

    fn prepare_transaction<D: TransactionData>(
        &self,
        transaction_data: &D,
    ) -> B::TransactionRequest {
        let call_data = transaction_data.into_transaction_data();
        let tx = B::TransactionRequest::default()
            .with_to(transaction_data.contract_address())
            .with_input(call_data);

        let tx = if let Some(value) = transaction_data.value() {
            tx.with_value(value)
        } else {
            tx
        };

        tx
    }

    pub async fn call_transaction<D: TransactionData>(
        &self,
        transaction_data: D,
    ) -> Result<(u64, D::CallOutput)> {
        let tx = self.prepare_transaction(&transaction_data);

        let gas_estimate = self.provider.estimate_gas(tx.clone()).await?;

        let response = self.provider.call(tx).await?;

        let decoded_output = transaction_data.decode_call_output(response)?;

        Ok((gas_estimate, decoded_output))
    }
}

impl<T: Provider<Ethereum>> RpcClient<Ethereum, T> {
    pub async fn send_transaction<D: TransactionData>(
        &self,
        transaction_data: D,
    ) -> Result<(u64, D::TransactionOutput)> {
        let tx = self.prepare_transaction(&transaction_data);

        let tx_hash = self
            .provider
            .send_transaction(tx)
            .await?
            .with_required_confirmations(2)
            .watch()
            .await?;

        let receipt = self
            .provider
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or(anyhow!("Transaction not found"))?;

        let tx_hash = receipt.transaction_hash();

        let event = receipt
            .logs()
            .into_iter()
            .filter(|log| log.transaction_hash == Some(tx_hash))
            .find_map(|log| {
                log.log_decode_validate::<D::Event>()
                    .ok()
                    .map(|l| transaction_data.decode_event(log.address(), l.data()))
            })
            .ok_or(anyhow!("Event not found in transaction logs"))?;

        Ok((receipt.gas_used, event))
    }
}
