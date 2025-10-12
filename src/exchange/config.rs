use crate::data::OrderReq;
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[async_trait]
pub trait RestClient {
    async fn place_order(&self, req: &OrderReq) -> Result<String, anyhow::Error>;
    async fn cancel_order(&self, req: &OrderReq) -> Result<String, anyhow::Error>;
}

pub async fn signature(secret_key: &[u8], msg: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret_key)
        .expect("HMAC can take key of any size..");
    mac.update(msg.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
