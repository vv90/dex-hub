use crate::{blockchain::BlockchainNetwork, multicall};
use alloy::primitives::Bytes;
use anyhow::Result;

pub trait MulticallData<B: BlockchainNetwork> {
    type Calls: IntoIterator<Item = multicall::Multicall3::Call>;
    type Output;

    fn size(&self) -> usize;
    fn to_calls(&self) -> Self::Calls;
    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output>;
}
