use std::{collections::HashSet, marker::PhantomData};

use alloy::{
    network::{Ethereum, NetworkWallet, ReceiptResponse, TransactionBuilder},
    primitives::{Address, FixedBytes, U256},
    providers::{
        Identity, Provider, ProviderBuilder, RootProvider,
        fillers::{FillProvider, JoinFill, RecommendedFillers},
    },
    rpc::types::Log,
    sol_types::SolCall,
};
use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use reqwest::{
    Url,
    header::{CONNECTION, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{self, Message, handshake::client::generate_key};

use crate::{
    blockchain::{BlockNumber, BlockchainNetwork},
    multicall,
    rpc::{call_data::CallData, multicall_data::MulticallData, transaction_data::TransactionData},
};

pub struct RpcClient<B: BlockchainNetwork, T: Provider<B>> {
    provider: T,
    _blockchain_marker: PhantomData<fn() -> B>,
}

#[derive(Debug, Deserialize)]
struct SubscriptionResponse {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct SubscriptionDataParams<T> {
    result: T,
}

#[derive(Debug, Deserialize)]
struct SubscriptionNotification<T> {
    params: SubscriptionDataParams<T>,
}

pub async fn subscribe_logs<B: BlockchainNetwork + RecommendedFillers>(
    topics: HashSet<FixedBytes<32>>,
) -> Result<mpsc::UnboundedReceiver<Log>> {
    let ws_url = env!("DRPC_WS_URL");
    let api_key = env!("DRPC_API_KEY");
    let (sender, receiver) = mpsc::unbounded_channel::<Log>();

    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .header(HOST, "lb.drpc.org")
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "Upgrade")
        .header(SEC_WEBSOCKET_KEY, generate_key())
        .header(SEC_WEBSOCKET_VERSION, "13")
        .uri(format!(
            "{}?network={}&dkey={}",
            ws_url,
            B::BLOCKCHAIN.name(),
            api_key
        ))
        .body(())?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();
    let sub_id = 1;
    let subscription_msg = json!({
        "jsonrpc": "2.0",
        "method": "eth_subscribe",
        // "params": ["newHeads"],
        // "params": [
        //     "logs",
        //     {"topics": [[
        //         format!("{}", functions_v3::SWAP_TOPIC),
        //         format!("{}", functions_v3::MINT_TOPIC),
        //         format!("{}", functions_v3::BURN_TOPIC)]
        //     }]],
        "params": [
            "logs",
            {
                "topics": [topics.into_iter().map(|topic| topic.to_string()).collect::<Vec<String>>()]
            }
        ],
        "id": sub_id
    })
    .to_string();

    println!("Subscription message: {}", subscription_msg);

    write
        .send(tungstenite::protocol::Message::Text(
            subscription_msg.into(),
        ))
        .await?;

    loop {
        match read.next().await {
            None => {
                println!(
                    "Connection closed before receiving subscription response ({})",
                    B::BLOCKCHAIN.name()
                );
                return Err(anyhow!(
                    "Connection closed before receiving subscription response ({})",
                    B::BLOCKCHAIN.name()
                ));
            }
            Some(Ok(Message::Text(msg))) => {
                match serde_json::from_slice::<SubscriptionResponse>(msg.as_bytes()) {
                    Ok(response) => {
                        if response.id == sub_id {
                            println!("Received subscription response: {:?}", response);
                            break;
                        } else {
                            println!("Received unexpected subscription id: {:?}", response);
                        }
                    }
                    Err(err) => {
                        println!("Failed to parse subscription response: {:?}", err);
                    }
                }
            }
            Some(msg) => {
                println!("Received unexpected message: {:?}", msg);
            }
        }
    }

    tokio::spawn(async move {
        println!("Listening for logs ({})", B::BLOCKCHAIN.name());
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(msg)) => {
                    match serde_json::from_slice::<SubscriptionNotification<Log>>(msg.as_bytes()) {
                        Ok(sub_notification) => {
                            if let Err(err) = sender.send(sub_notification.params.result) {
                                println!("Failed to send update: {}", err);
                            }
                        }
                        Err(err) => {
                            println!("Failed to parse log: {}", err);
                            println!("{}", msg);
                        }
                    }
                }
                Ok(Message::Ping(p)) => {
                    if let Err(e) = write.send(Message::Pong(p)).await {
                        println!("Error sending pong: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("WebSocket connection closed");
                    break;
                }
                Ok(_) => {
                    println!("Received non-text message");
                }
                Err(e) => {
                    println!("Error receiving message: {}", e);
                }
            }
        }
    });

    Ok(receiver)
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
    ) -> Result<Vec<Result<D::Output, D::DecodeError>>> {
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
            (0, Vec::<Result<D::Output, D::DecodeError>>::new()),
            |(position, mut collection), data| -> (usize, Vec<Result<D::Output, D::DecodeError>>) {
                let next_position = position + data.size();
                let item =
                    data.decode_output(&decoded_response.returnData[position..next_position]);
                collection.push(item);
                (next_position, collection)
            },
        );

        Ok(results)
    }

    pub async fn get_multicall_chunked<D: MulticallData<B>>(
        &self,
        calls_data: &[D],
        block_number: BlockNumber<B>,
        chunk_size: usize,
    ) -> Result<Vec<Result<D::Output, D::DecodeError>>> {
        let results = futures_util::stream::iter(calls_data.chunks(chunk_size))
            .map(anyhow::Ok)
            .try_fold(
                Vec::<Result<D::Output, D::DecodeError>>::new(),
                async |mut combined_results, chunk| {
                    let results = self.get_multicall(chunk, block_number).await?;
                    combined_results.extend(results);
                    Ok(combined_results)
                },
            )
            .await?;

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
