use alloy::{
    primitives::{Address, address},
    sol,
};

use crate::blockchain::Blockchain;

// pub const fn router_address(blockchain: Blockchain) -> Address {
//     match blockchain {
//         Blockchain::Ethereum => address!("0x80226fc0Ee2b096224EeAc085Bb9a8cba1146f7D"),
//         Blockchain::BSC => address!("0x34B03Cb9086d7D758AC55af71584F81A598759FE"),
//         Blockchain::Arbitrum => address!("0x141fa059441E0ca23ce184B6A78bafD2A517DdE8"),
//     }
// }

pub const fn token_admin_registry_address(blockchain: Blockchain) -> Address {
    match blockchain {
        Blockchain::Ethereum => address!("0xb22764f98dD05c789929716D677382Df22C05Cb6"),
        Blockchain::BSC => address!("0x736Fd8660c443547a85e4Eaf70A49C1b7Bb008fc"),
        Blockchain::Arbitrum => address!("0x39AE1032cF4B334a1Ed41cdD0833bdD7c7E7751E"),
    }
}

sol! {
    contract TokenAdminRegistry {
        /// @notice Returns all pools for the given tokens.
        /// @dev Will return address(0) for tokens that do not have a pool.
        function getPools(address[] calldata tokens) external view returns (address[] memory);

        /// @notice Returns a list of tokens that are configured in the token admin registry.
        /// @param startIndex Starting index in list, can be 0 if you want to start from the beginning.
        /// @param maxCount Maximum number of tokens to retrieve. Since the list can be large,
        /// it is recommended to use a paging mechanism to retrieve all tokens. If querying for very
        /// large lists, RPCs can time out. If you want all tokens, use type(uint64).max.
        /// @return tokens List of configured tokens.
        /// @dev The function is paginated to avoid RPC timeouts.
        /// @dev The ordering is guaranteed to remain the same as it is not possible to remove tokens
        /// from s_tokens.
        function getAllConfiguredTokens(uint64 startIndex, uint64 maxCount) external view returns (address[] memory tokens);
    }

    contract TokenPool {
        /// @notice Gets the pool address on the remote chain.
        /// @param remoteChainSelector Remote chain selector.
        /// @dev To support non-evm chains, this value is encoded into bytes
        function getRemotePools( uint64 remoteChainSelector ) public view returns (bytes[] memory);

        /// @notice Gets the token address on the remote chain.
        /// @param remoteChainSelector Remote chain selector.
        /// @dev To support non-evm chains, this value is encoded into bytes
        function getRemoteToken( uint64 remoteChainSelector ) public view returns (bytes memory);
    }
}
