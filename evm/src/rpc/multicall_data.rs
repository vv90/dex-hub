use crate::{blockchain::BlockchainNetwork, multicall};
use alloy::primitives::Bytes;
use anyhow::Result;

pub trait MulticallData<B: BlockchainNetwork> {
    const SIZE: usize;
    type Calls: IntoIterator<Item = multicall::Multicall3::Call>;
    type Output;

    fn to_calls(&self) -> Self::Calls;
    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output>;
}
