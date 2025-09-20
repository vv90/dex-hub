use solana_sdk::pubkey::Pubkey;

pub struct BaseConfig {
    // Token configuration
    pub token_program: Pubkey,      // SPL Token or Token-2022 program ID
    pub mint: Pubkey,               // Token mint address
    pub decimals: u8,               // Token decimals
    pub pool_signer: Pubkey,        // Pool signer PDA
    pub pool_token_account: Pubkey, // Pool's associated token account

    // Ownership and administration
    pub owner: Pubkey,            // Current pool owner
    pub proposed_owner: Pubkey,   // Proposed new owner (for ownership transfer)
    pub rate_limit_admin: Pubkey, // Rate limit administrator (currently unused - rate limits managed by pool owner)

    // CCIP integration
    pub router_onramp_authority: Pubkey, // Router's onramp authority PDA
    pub router: Pubkey,                  // CCIP Router program address
    pub rmn_remote: Pubkey,              // RMN Remote program address

    // Lock-Release specific (unused in BurnMint pools)
    pub rebalancer: Pubkey, // Rebalancer address for liquidity management
    pub can_accept_liquidity: bool, // Whether pool accepts liquidity operations

    // Access control
    pub list_enabled: bool,      // Whether allowlist is enabled
    pub allow_list: Vec<Pubkey>, // Allowlisted addresses for pool operations
}
