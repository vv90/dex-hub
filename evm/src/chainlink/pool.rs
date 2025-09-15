use std::marker::PhantomData;

use alloy::primitives::Address;

use crate::blockchain::BlockchainNetwork;

#[derive(Debug, Clone, Copy)]
pub struct PoolAddress<B: BlockchainNetwork>(pub(crate) Address, PhantomData<B>);

impl<B: BlockchainNetwork> PoolAddress<B> {
    pub fn new(address: Address) -> Self {
        PoolAddress(address, PhantomData)
    }

    pub fn address(&self) -> Address {
        self.0
    }

    // pub fn blockchain(&self) -> Blockchain {
    //     B::BLOCKCHAIN
    // }
}
