use alloy::{
    primitives::{Address, Bytes, U256},
    sol_types::SolEvent,
};
use anyhow::Result;

pub trait TransactionData {
    type CallOutput;
    type TransactionOutput;
    type Event: SolEvent;

    fn contract_address(&self) -> Address;
    fn value(&self) -> Option<U256>;
    fn into_transaction_data(&self) -> Bytes;
    fn decode_call_output(&self, response: Bytes) -> Result<Self::CallOutput>;
    fn decode_event(&self, address: Address, event: &Self::Event) -> Self::TransactionOutput;
}
