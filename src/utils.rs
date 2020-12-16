use super::*;
use actix_web::{error, HttpResponse};
use bson::Document;
use mongodb::Cursor;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BusinessError {
    #[error("10001#字段上的验证错误: {field}")]
    ValidationError { field: String },
    #[error("10002#参数错误")]
    ArgumentError,
    #[error("10000#发生内部错误。请稍后再试")]
    InternalError {
        #[source]
        source: anyhow::Error,
    },
}

impl BusinessError {
    #[allow(dead_code)]
    fn to_code(&self) -> i32 {
        let code = &self.to_string()[0..5];
        code.parse().unwrap_or(-1)
    }

    #[allow(dead_code)]
    fn to_message(&self) -> String {
        self.to_string()[6..].to_owned()
    }
}

impl error::ResponseError for BusinessError {
    fn error_response(&self) -> HttpResponse {
        let resp = Resp::err(self.to_code(), &self.to_message());
        HttpResponse::BadRequest().json(resp)
    }
}

impl From<mongodb::error::Error> for BusinessError {
    fn from(e: mongodb::error::Error) -> Self {
        log::error!("mongodb error, {}", e.to_owned());
        BusinessError::InternalError { source: anyhow!(e) }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Resp<T>
where
    T: Serialize,
{
    // 操作是否成功 true.成功 / false.失败
    success: bool,
    // 响应信息
    messages: String,
    // 页数
    page: u64,
    // 每页最大值
    page_size: u64,
    // 查询内容总数
    total: u64,
    code: i32,
    // 数据内容
    data: Option<T>,
}

impl<T: Serialize> Resp<T> {
    #[allow(dead_code)]
    pub(crate) fn ok(data: T, message: &str, page: u64, page_size: u64, total: u64) -> Self {
        Resp {
            success: true,
            code: 200,
            page,
            page_size,
            total,
            messages: message.to_owned(),
            data: Some(data),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn to_json_result(&self) -> Result<HttpResponse, BusinessError> {
        Ok(HttpResponse::Ok().json(self))
    }
}

impl Resp<()> {
    pub(crate) fn err(error: i32, message: &str) -> Self {
        Resp {
            success: false,
            code: error,
            page: 0,
            page_size: 0,
            total: 0,
            messages: message.to_owned(),
            data: None,
        }
    }
}

pub trait CursorAsVec {
    fn as_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T>;
}

impl CursorAsVec for Cursor {
    fn as_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T> {
        self.map(|item| {
            let doc: Document = item.unwrap();
            let bson = bson::Bson::Document(doc);
            return bson::from_bson(bson).unwrap();
        })
        .collect()
    }
}
