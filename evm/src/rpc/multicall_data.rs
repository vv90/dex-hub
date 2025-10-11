use crate::{blockchain::BlockchainNetwork, multicall};
use alloy::primitives::Bytes;

pub trait MulticallData<B: BlockchainNetwork> {
    type Calls: IntoIterator<Item = multicall::Multicall3::Call>;
    type Output;
    type DecodeError: std::error::Error;

    fn size(&self) -> usize;
    fn to_calls(&self) -> Self::Calls;
    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output, Self::DecodeError>;
}
