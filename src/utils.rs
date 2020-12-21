use super::*;
use actix_web::{error, HttpResponse};
use bson::ordered::OrderedDocument;
use bson::Document;
use md5;
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
    #[inline]
    pub fn ok(data: T, message: &str, page: u64, page_size: u64, total: u64) -> Self {
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
    pub fn to_json_result(&self) -> Result<HttpResponse, BusinessError> {
        Ok(HttpResponse::Ok().json(self))
    }
}

impl Resp<()> {
    #[allow(dead_code)]
    #[inline]
    pub fn err(error: i32, message: &str) -> Self {
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

// pub trait OrderedDocumentAsStruct {
//     fn as_struct<'a, T: Serialize + Deserialize<'a>>(&mut self);
// }

// impl OrderedDocumentAsStruct for OrderedDocument {
//     fn as_struct<'a, T: Serialize + Deserialize<'a>>(&mut self) {
//         let keys = self.keys();
//         let r: Vec<String> = keys
//             .filter(|k| self.is_null(k))
//             .map(|x| x.to_owned())
//             .collect();
//         // info!("r = {:?}", r);
//     }
// }

/// 安全密鑰
const SECRET_KEYS: &str = "!s@w4$qS%^(_123-=0Xha9452sLW^%sfa9)\\";

/// md5
#[inline]
pub fn md5_str(content: &str) -> String {
    let encrypt = md5::compute(content);
    format!("{:x}", encrypt)
}

/// 自定义 安全密钥 生成密码
#[inline]
pub fn get_password(real_password: &str, secret_key: &str, secret: &str) -> String {
    let origin = format!("{}-{}-{}", real_password, secret_key, secret);
    md5_str(origin.as_str())
}

/// 使用默认 安全密钥 生成密码
#[inline]
pub fn get_password_default(real_password: &str, secret: &str) -> String {
    get_password(real_password, SECRET_KEYS, secret)
}

/// 结构体转mongodb文档
#[inline]
pub fn struct_to_document<'a, T: Sized + Serialize + Deserialize<'a>>(
    t: &T,
) -> Option<OrderedDocument> {
    let mid: Option<OrderedDocument> = bson::to_bson(t)
        .ok()
        .map(|x| x.as_document().unwrap().to_owned());

    mid.map(|mut doc| {
        let keys = doc.keys();
        let rm: Vec<String> = keys
            .filter(|k| doc.is_null(k))
            .map(|x| x.to_owned())
            .collect();
        for x in rm {
            doc.remove(&x);
        }
        doc
    })
}
