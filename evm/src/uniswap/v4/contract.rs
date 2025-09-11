use alloy::{
    primitives::{Address, address},
    sol,
};

use crate::blockchain::Blockchain;

pub const fn state_view_address(blockchain: Blockchain) -> Address {
    match blockchain {
        Blockchain::Ethereum => address!("0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227"),
        Blockchain::BSC => address!("0xd13dd3d6e93f276fafc9db9e6bb47c1180aee0c4"),
        Blockchain::Arbitrum => address!("0x76fd297e2d437cd7f76d50f01afe6160f86e9990"),
    }
}

sol! {
    type PoolId is bytes32;

    function getSlot0(PoolId poolId)
        external
        view
        returns (uint160 sqrtPriceX96, int24 tick, uint24 protocolFee, uint24 lpFee);

    function getLiquidity(PoolId poolId) external view returns (uint128 liquidity);

    function getTickBitmap(PoolId poolId, int16 tick) external view returns (uint256 tickBitmap);

    function getTickInfo(PoolId poolId, int24 tick)
        external
        view
        returns (
            uint128 liquidityGross,
            int128 liquidityNet,
            uint256 feeGrowthOutside0X128,
            uint256 feeGrowthOutside1X128
        );

    function getTickLiquidity(PoolId poolId, int24 tick)
        external
        view
        returns (uint128 liquidityGross, int128 liquidityNet);
}
