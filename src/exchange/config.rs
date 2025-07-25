use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Exchangecfg {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub kucoin: Exchangecfg,
    pub binance: Exchangecfg,
    pub paper: bool
}