use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn signature(secret_key: &[u8], msg: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret_key)
        .expect("HMAC can take keys of any size");
    mac.update(msg.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}