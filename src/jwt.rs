use super::*;
use crate::date_time;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{de, ser, Serialize};

/// secret key
pub const SECRET_KEY: &[u8] = b"ynos";
/// 有效期一天
pub const DAY_ONE: u64 = 86400;

/// 默认结构 用户token
#[derive(Deserialize, Serialize, Debug)]
pub struct UserToken {
    /// 过期时间
    pub exp: u64,
    /// 用户id
    pub id: String,
    /// 用户名称
    pub name: String,
}

/// 生成token 默认模式
/// # Examples
/// ```rust
/// let key = b"ynos";
/// let token = encode_by("id","name");
/// ```
pub fn encode(id: &str, name: &str) -> String {
    let now = date_time::timestamp();
    let playload = UserToken {
        exp: now + DAY_ONE,
        id: id.to_string(),
        name: name.to_string(),
    };
    jsonwebtoken::encode(
        &Header::default(),
        &playload,
        &EncodingKey::from_secret(&*SECRET_KEY),
    )
    .unwrap()
}

/// 生成自定义token
/// # Examples
/// ```rust
/// #[derive(Deserialize, Serialize)]
/// let struct usertoken {
///    exp:u64,
///    id:String,
/// }
/// let key = b"ynos";
/// let user = usertoken {exp:1000000,id:"String"};
/// let token = encode_by(user,&key);
/// ```
pub fn encode_by<T: ser::Serialize>(data: &T, secret_key: &[u8]) -> String {
    jsonwebtoken::encode::<T>(
        &Header::default(),
        &data,
        &EncodingKey::from_secret(&*secret_key),
    )
    .unwrap()
}

/// 解码authorization字段 - 默认方式
pub fn decode(token: &str) -> jsonwebtoken::errors::Result<TokenData<UserToken>> {
    jsonwebtoken::decode::<UserToken>(
        token,
        &DecodingKey::from_secret(&*SECRET_KEY),
        &Validation::default(),
    )
}

/// 解码authorization字段 - 自定义方式
pub fn decode_by<T: de::DeserializeOwned>(
    token: &str,
    secret_key: &[u8],
) -> jsonwebtoken::errors::Result<TokenData<T>> {
    jsonwebtoken::decode::<T>(
        token,
        &DecodingKey::from_secret(&*secret_key),
        &Validation::default(),
    )
}
