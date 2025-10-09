use std::{
    collections::HashMap,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::Path,
};

use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use serde::{Deserialize, de::DeserializeOwned};

use crate::tokens::{TokenAddress, TokenInfo};

#[derive(Debug, Clone, Deserialize)]
struct SubgraphResponseData<T> {
    data: T,
}

async fn query_subgraph<T: DeserializeOwned>(
    url: &str,
    query: &str,
    cache_path: &str,
) -> Result<T> {
    let api_key = env!("SUBGRAPH_API_KEY");

    let mut hasher = std::hash::DefaultHasher::new();
    query.hash(&mut hasher);
    let query_hash = format!("{:x}", hasher.finish());

    let path = Path::new(cache_path);
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    let cache_file = format!("{}/{}.json", cache_path, query_hash);

    let cached_response = File::open(&cache_file).and_then(|mut file| {
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(buf)
    });

    match cached_response {
        Ok(response_text) => {
            // println!(
            //     "Using cached response for first: {}, skip: {}",
            //     chunk_size, skip
            // );
            let response_data = serde_json::from_str::<SubgraphResponseData<T>>(&response_text)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to parse cached response {}: {:?}", query_hash, e)
                })?;

            Ok(response_data.data)
        }
        Err(_) => {
            println!("{}", url);
            println!("{}", query);
            let response = reqwest::Client::new()
                .post(url)
                .header("Content-Type", "application/json")
                .bearer_auth(api_key)
                .body(format!(
                    "{{\"query\": \"{}\", \"operationName\": \"Subgraphs\", \"variables\": {{}} }}",
                    query
                ))
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send subgraph request: {:?}", e))?;

            match response.status() {
                reqwest::StatusCode::OK => {
                    let response_text = response.text().await?;

                    let handle_parsing_error = |e: serde_json::Error| -> anyhow::Error {
                        match File::create(format!("{}/{}.failed.json", cache_path, query_hash))
                            .and_then(|mut file| file.write_all(response_text.as_bytes()))
                        {
                            Ok(_) => anyhow::anyhow!("Failed to parse response: {:?}", e,),
                            Err(err) => anyhow!(
                                "Failed to parse response: {:?}\nFailed to write to file: {:?}",
                                e,
                                err
                            ),
                        }
                    };

                    let response_data =
                        serde_json::from_str::<SubgraphResponseData<T>>(&response_text)
                            .map_err(handle_parsing_error)?;

                    if let Err(e) = File::create(&cache_file)
                        .and_then(|mut file| file.write_all(response_text.as_bytes()))
                    {
                        println!("Failed to write cache file {}: {}", cache_file, e);
                    }

                    Ok(response_data.data)
                }
                code => {
                    println!("Error {}: {}", code, response.status());
                    Err(anyhow::anyhow!("Failed to fetch data from subgraph"))
                }
            }
        }
    }
}

pub struct SubgraphQueryParams {
    pub limit: u32,
    pub skip: u32,
    pub min_value: Decimal,
}

pub struct SubgraphConfig<D: DeserializeOwned, P> {
    pub subgraph_url: &'static str,
    pub subgraph_name: &'static str,
    pub format_query: fn(SubgraphQueryParams) -> String,
    // deserialised data from subgraph response is mapped into pools,
    // updating the tokens hash map with new tokens
    pub map_pools:
        fn(HashMap<TokenAddress, TokenInfo>, D) -> (Vec<P>, HashMap<TokenAddress, TokenInfo>),
}

// Since many pools may refer to the same tokens,
// we want to return pools and tokens in those pools separately
// to avoid data duplication.
// Pool data type is only supposed to have token address and not full token data
impl<D: DeserializeOwned, P> SubgraphConfig<D, P> {
    pub async fn query_pools(
        &self,
        min_value: Decimal,
    ) -> Result<(Vec<P>, HashMap<TokenAddress, TokenInfo>)> {
        let cache_path = format!(
            "{}/{}",
            env!("SUBGRAPH_RESPONSE_CACHE_PATH"),
            self.subgraph_name
        );

        let limit = 1000;

        let mut all_pools = Vec::<P>::new();
        let mut all_tokens = HashMap::<TokenAddress, TokenInfo>::new();
        let mut skip = 0;

        loop {
            let params = SubgraphQueryParams {
                limit,
                skip,
                min_value,
            };
            let query = (self.format_query)(params);

            let pools_data = query_subgraph::<D>(self.subgraph_url, &query, &cache_path).await?;

            let (pools, tokens) = (self.map_pools)(all_tokens, pools_data);
            all_tokens = tokens;

            let items_loaded = pools.len();
            if items_loaded == 0 {
                break; // No more pools to load
            } else {
                all_pools.extend(pools);
                // println!("Loaded {} pools", pools_heap.len());
                skip += items_loaded as u32;
            }
        }

        Ok((all_pools, all_tokens))
    }
}
