use alloy::{
    primitives::{Address, address},
    sol,
};

use crate::blockchain::Blockchain;

pub const fn multicall3_address(blockchain: Blockchain) -> Address {
    match blockchain {
        Blockchain::Ethereum => address!("0xcA11bde05977b3631167028862bE2a173976CA11"),
        Blockchain::BSC => address!("0xcA11bde05977b3631167028862bE2a173976CA11"),
        Blockchain::Arbitrum => address!("0xcA11bde05977b3631167028862bE2a173976CA11"),
    }
}

sol! {
    contract Multicall3 {
        struct Call {
            address target;
            bytes callData;
        }

        struct Call3 {
            address target;
            bool allowFailure;
            bytes callData;
        }

        struct Call3Value {
            address target;
            bool allowFailure;
            uint256 value;
            bytes callData;
        }

        struct Result {
            bool success;
            bytes returnData;
        }
        /// @notice Backwards-compatible call aggregation with Multicall
        /// @param calls An array of Call structs
        /// @return blockNumber The block number where the calls were executed
        /// @return returnData An array of bytes containing the responses
        function aggregate(Call[] calldata calls) public payable returns (uint256 blockNumber, bytes[] memory returnData);
        /// @notice Backwards-compatible with Multicall2
        /// @notice Aggregate calls without requiring success
        /// @param requireSuccess If true, require all calls to succeed
        /// @param calls An array of Call structs
        /// @return returnData An array of Result structs
        function tryAggregate(bool requireSuccess, Call[] calldata calls) public payable returns (Result[] memory returnData);
        /// @notice Backwards-compatible with Multicall2
        /// @notice Aggregate calls and allow failures using tryAggregate
        /// @param calls An array of Call structs
        /// @return blockNumber The block number where the calls were executed
        /// @return blockHash The hash of the block where the calls were executed
        /// @return returnData An array of Result structs
        function blockAndAggregate(Call[] calldata calls) public payable returns (uint256 blockNumber, bytes32 blockHash, Result[] memory returnData);
        /// @notice Backwards-compatible with Multicall2
        /// @notice Aggregate calls and allow failures using tryAggregate
        /// @param calls An array of Call structs
        /// @return blockNumber The block number where the calls were executed
        /// @return blockHash The hash of the block where the calls were executed
        /// @return returnData An array of Result structs
        function tryBlockAndAggregate(bool requireSuccess, Call[] calldata calls) public payable returns (uint256 blockNumber, bytes32 blockHash, Result[] memory returnData);
        /// @notice Aggregate calls, ensuring each returns success if required
        /// @param calls An array of Call3 structs
        /// @return returnData An array of Result structs
        function aggregate3(Call3[] calldata calls) public payable returns (Result[] memory returnData);
        /// @notice Aggregate calls with a msg value
        /// @notice Reverts if msg.value is less than the sum of the call values
        /// @param calls An array of Call3Value structs
        /// @return returnData An array of Result structs
        function aggregate3Value(Call3Value[] calldata calls) public payable returns (Result[] memory returnData);
    }
}
