use crate::{
    rpc::transaction_data::TransactionData,
    tokens::TokenAddress,
    uniswap_internal::v3::{contract, deployments, pool::Fee},
};
use alloy::{
    primitives::{Address, Bytes, FixedBytes, I256, U256},
    sol_types::SolCall,
};

pub struct SwapPath {
    pub path: Vec<(TokenAddress, Fee)>,
    pub output: TokenAddress,
}

fn fee_as_path_bytes(fee: Fee) -> [u8; 3] {
    let mut bytes = [0u8; 3];
    bytes.copy_from_slice(&(fee as u32).to_be_bytes()[1..4]);
    bytes
}

pub struct Swap {
    pub path: SwapPath,
    pub amount: u128,
    pub recipient: Address,
}

impl TransactionData for Swap {
    type CallOutput = U256;
    type TransactionOutput = (Address, I256, I256);
    type Event = contract::IUniswapV3PoolEvents::Swap;

    fn contract_address(&self) -> Address {
        deployments::SWAP_ROUTER2_ADDRESS
    }

    fn value(&self) -> Option<U256> {
        None
    }

    fn into_transaction_data(&self) -> Bytes {
        let SwapPath { path, output } = &self.path;

        let TokenAddress(Address(FixedBytes(output_token_bytes)), _) = output;

        let path_bytes = Bytes::from_iter(
            path.iter()
                .flat_map(
                    |(TokenAddress(Address(FixedBytes(address_bytes)), _), fee)| {
                        address_bytes
                            .iter()
                            .copied()
                            .chain(fee_as_path_bytes(*fee).into_iter())
                    },
                )
                .chain(output_token_bytes.iter().copied()),
        );

        let params = contract::SwapRouter2::ExactInputParams {
            path: path_bytes,
            recipient: self.recipient,
            amountIn: U256::from(self.amount),
            amountOutMinimum: U256::ZERO,
        };

        contract::SwapRouter2::exactInputCall { params }
            .abi_encode()
            .into()
    }

    fn decode_call_output(
        &self,
        response: alloy::primitives::Bytes,
    ) -> anyhow::Result<Self::CallOutput> {
        let output = contract::SwapRouter::exactInputCall::abi_decode_returns(&response)?;
        Ok(output)
    }

    fn decode_event(&self, address: Address, event: &Self::Event) -> Self::TransactionOutput {
        (address, event.amount0, event.amount1)
    }
}
