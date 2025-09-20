pub struct RateLimitConfig {
    pub enabled: bool, // Whether rate limiting is enabled
    pub capacity: u64, // Maximum tokens in bucket
    pub rate: u64,     // Tokens per second refill rate
}

pub struct RateLimitTokenBucket {
    pub tokens: u64,       // Current tokens in bucket
    pub last_updated: u64, // Last refill timestamp
    cfg: RateLimitConfig,  // Rate limit configuration
}

pub struct RemoteAddress {
    pub address: Vec<u8>, // Address bytes (max 64 bytes)
}

pub struct RemoteConfig {
    pub pool_addresses: Vec<RemoteAddress>, // Remote pool addresses (supports multiple versions)
    pub token_address: RemoteAddress,       // Remote token address
    pub decimals: u8,                       // Remote token decimals
}

pub struct BaseChain {
    pub remote: RemoteConfig, // Remote chain token and pool configuration
    pub inbound_rate_limit: RateLimitTokenBucket, // Rate limiting for incoming transfers
    pub outbound_rate_limit: RateLimitTokenBucket, // Rate limiting for outgoing transfers
}
