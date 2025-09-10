use std::marker::PhantomData;

use alloy::{
    network::{
        AnyNetwork, BuildResult, Network, NetworkWallet, TransactionBuilder,
        TransactionBuilderError, UnbuiltTransactionError,
    },
    primitives::{Address, Bytes, ChainId, TxKind, U256},
    providers::fillers::RecommendedFillers,
    rpc::types::{AccessList, TransactionInputKind, TransactionRequest},
    serde::WithOtherFields,
};

#[derive(Debug, Clone, Copy)]
pub struct EVMNetwork<T: Send + Sync + std::fmt::Debug + Clone + Copy + 'static> {
    _phantom: PhantomData<T>,
}

#[derive(Clone, Copy, Debug)]
pub struct BSCNetwork;

#[derive(Debug, Clone, Copy)]
pub struct ArbitrumNetwork;

pub type Ethereum = alloy::network::Ethereum;
pub type Arbitrum = EVMNetwork<ArbitrumNetwork>;
pub type BSC = EVMNetwork<BSCNetwork>;

fn map_transaction_builder_error<T: Send + Sync + std::fmt::Debug + Clone + Copy + 'static>(
    error: TransactionBuilderError<AnyNetwork>,
) -> TransactionBuilderError<EVMNetwork<T>> {
    match error {
        TransactionBuilderError::InvalidTransactionRequest(tx_type, val) => {
            TransactionBuilderError::InvalidTransactionRequest(tx_type, val)
        }
        TransactionBuilderError::UnsupportedSignatureType => {
            TransactionBuilderError::UnsupportedSignatureType
        }
        TransactionBuilderError::Signer(e) => TransactionBuilderError::Signer(e),
        TransactionBuilderError::Custom(e) => TransactionBuilderError::Custom(e),
    }
}

impl<M: Send + Sync + std::fmt::Debug + Clone + Copy + 'static> RecommendedFillers
    for EVMNetwork<M>
{
    type RecommendedFillers = <AnyNetwork as RecommendedFillers>::RecommendedFillers;

    fn recommended_fillers() -> Self::RecommendedFillers {
        AnyNetwork::recommended_fillers()
    }
}

impl<M: Send + Sync + std::fmt::Debug + Clone + Copy + 'static> Network for EVMNetwork<M> {
    type TxType = <AnyNetwork as Network>::TxType;
    type TxEnvelope = <AnyNetwork as Network>::TxEnvelope;
    type UnsignedTx = <AnyNetwork as Network>::UnsignedTx;
    type ReceiptEnvelope = <AnyNetwork as Network>::ReceiptEnvelope;
    type Header = <AnyNetwork as Network>::Header;
    type TransactionRequest = <AnyNetwork as Network>::TransactionRequest;
    type TransactionResponse = <AnyNetwork as Network>::TransactionResponse;
    type ReceiptResponse = <AnyNetwork as Network>::ReceiptResponse;
    type HeaderResponse = <AnyNetwork as Network>::HeaderResponse;
    type BlockResponse = <AnyNetwork as Network>::BlockResponse;
}

impl<M: Send + Sync + std::fmt::Debug + Clone + Copy + 'static> TransactionBuilder<EVMNetwork<M>>
    for WithOtherFields<TransactionRequest>
{
    fn chain_id(&self) -> Option<ChainId> {
        <Self as TransactionBuilder<AnyNetwork>>::chain_id(self)
    }

    fn set_chain_id(&mut self, chain_id: ChainId) {
        <Self as TransactionBuilder<AnyNetwork>>::set_chain_id(self, chain_id);
    }

    fn nonce(&self) -> Option<u64> {
        <Self as TransactionBuilder<AnyNetwork>>::nonce(self)
    }

    fn set_nonce(&mut self, nonce: u64) {
        <Self as TransactionBuilder<AnyNetwork>>::set_nonce(self, nonce);
    }

    fn take_nonce(&mut self) -> Option<u64> {
        <Self as TransactionBuilder<AnyNetwork>>::take_nonce(self)
    }

    fn input(&self) -> Option<&Bytes> {
        <Self as TransactionBuilder<AnyNetwork>>::input(self)
    }

    fn set_input<T: Into<Bytes>>(&mut self, input: T) {
        <Self as TransactionBuilder<AnyNetwork>>::set_input(self, input);
    }

    fn set_input_kind<T: Into<Bytes>>(&mut self, input: T, kind: TransactionInputKind) {
        <Self as TransactionBuilder<AnyNetwork>>::set_input_kind(self, input, kind);
    }

    fn from(&self) -> Option<Address> {
        <Self as TransactionBuilder<AnyNetwork>>::from(self)
    }

    fn set_from(&mut self, from: Address) {
        <Self as TransactionBuilder<AnyNetwork>>::set_from(self, from);
    }

    fn kind(&self) -> Option<TxKind> {
        <Self as TransactionBuilder<AnyNetwork>>::kind(self)
    }

    fn clear_kind(&mut self) {
        <Self as TransactionBuilder<AnyNetwork>>::clear_kind(self);
    }

    fn set_kind(&mut self, kind: TxKind) {
        <Self as TransactionBuilder<AnyNetwork>>::set_kind(self, kind);
    }

    fn value(&self) -> Option<U256> {
        <Self as TransactionBuilder<AnyNetwork>>::value(self)
    }

    fn set_value(&mut self, value: U256) {
        <Self as TransactionBuilder<AnyNetwork>>::set_value(self, value);
    }

    fn gas_price(&self) -> Option<u128> {
        <Self as TransactionBuilder<AnyNetwork>>::gas_price(self)
    }

    fn set_gas_price(&mut self, gas_price: u128) {
        <Self as TransactionBuilder<AnyNetwork>>::set_gas_price(self, gas_price);
    }

    fn max_fee_per_gas(&self) -> Option<u128> {
        <Self as TransactionBuilder<AnyNetwork>>::max_fee_per_gas(self)
    }

    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: u128) {
        <Self as TransactionBuilder<AnyNetwork>>::set_max_fee_per_gas(self, max_fee_per_gas);
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        <Self as TransactionBuilder<AnyNetwork>>::max_priority_fee_per_gas(self)
    }

    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: u128) {
        <Self as TransactionBuilder<AnyNetwork>>::set_max_priority_fee_per_gas(
            self,
            max_priority_fee_per_gas,
        );
    }

    fn gas_limit(&self) -> Option<u64> {
        <Self as TransactionBuilder<AnyNetwork>>::gas_limit(self)
    }

    fn set_gas_limit(&mut self, gas_limit: u64) {
        <Self as TransactionBuilder<AnyNetwork>>::set_gas_limit(self, gas_limit);
    }

    /// Get the EIP-2930 access list for the transaction.
    fn access_list(&self) -> Option<&AccessList> {
        <Self as TransactionBuilder<AnyNetwork>>::access_list(self)
    }

    /// Sets the EIP-2930 access list.
    fn set_access_list(&mut self, access_list: AccessList) {
        <Self as TransactionBuilder<AnyNetwork>>::set_access_list(self, access_list);
    }

    fn complete_type(&self, ty: <AnyNetwork as Network>::TxType) -> Result<(), Vec<&'static str>> {
        <Self as TransactionBuilder<AnyNetwork>>::complete_type(self, ty)
    }

    fn can_submit(&self) -> bool {
        <Self as TransactionBuilder<AnyNetwork>>::can_submit(self)
    }

    fn can_build(&self) -> bool {
        <Self as TransactionBuilder<AnyNetwork>>::can_build(self)
    }

    #[doc(alias = "output_transaction_type")]
    fn output_tx_type(&self) -> <AnyNetwork as Network>::TxType {
        <Self as TransactionBuilder<AnyNetwork>>::output_tx_type(self)
    }

    #[doc(alias = "output_transaction_type_checked")]
    fn output_tx_type_checked(&self) -> Option<<AnyNetwork as Network>::TxType> {
        <Self as TransactionBuilder<AnyNetwork>>::output_tx_type_checked(self)
    }

    fn prep_for_submission(&mut self) {
        <Self as TransactionBuilder<AnyNetwork>>::prep_for_submission(self)
    }

    fn build_unsigned(self) -> BuildResult<<EVMNetwork<M> as Network>::UnsignedTx, EVMNetwork<M>> {
        <Self as TransactionBuilder<AnyNetwork>>::build_unsigned(self).map_err(|err| {
            UnbuiltTransactionError {
                request: err.request,
                error: map_transaction_builder_error(err.error),
            }
        })
    }
    async fn build<W: NetworkWallet<EVMNetwork<M>>>(
        self,
        wallet: &W,
    ) -> Result<<EVMNetwork<M> as Network>::TxEnvelope, TransactionBuilderError<EVMNetwork<M>>>
    {
        Ok(wallet.sign_request(self).await?)
    }
}
