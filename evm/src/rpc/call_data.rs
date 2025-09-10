use alloy::primitives::{Address, Bytes};
use anyhow::Result;

use crate::blockchain::BlockchainNetwork;

pub trait CallData<B: BlockchainNetwork> {
    type Output;

    fn contract_address(&self) -> Address;
    fn into_call_data(&self) -> Bytes;
    fn decode_call_output(&self, response: Bytes) -> Result<Self::Output>;
}
