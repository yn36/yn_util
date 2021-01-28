use super::*;
use actix_web::{error, HttpResponse};
use bson::{doc, oid::ObjectId, Document};
use futures::StreamExt;
use md5;
use mongodb::Cursor;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BusinessError {
    #[error("字段上的验证错误: {field}")]
    ValidationError { field: String },
    #[error("参数错误: {source}")]
    ArgumentError {
        #[source]
        source: anyhow::Error,
    },
    #[error("{source}")]
    InternalError {
        #[source]
        source: anyhow::Error,
    },
    #[error("用户未认证")]
    Unauthorized,
}

impl BusinessError {
    #[allow(dead_code)]
    fn to_code(&self) -> i32 {
        let code = &self.to_string()[0..5];
        code.parse().unwrap_or(-1)
    }

    #[allow(dead_code)]
    fn to_message(&self) -> String {
        self.to_string().to_owned()
        // self.to_string()[6..].to_owned()
    }
}

impl error::ResponseError for BusinessError {
    fn error_response(&self) -> HttpResponse {
        match self {
            BusinessError::ValidationError { field: _ } => {
                let resp = Resp::err(400, &self.to_message());
                HttpResponse::BadRequest().json(resp)
            }
            BusinessError::ArgumentError { source: _ } => {
                let resp = Resp::err(400, &self.to_message());
                HttpResponse::BadRequest().json(resp)
            }
            BusinessError::InternalError { source: _ } => {
                let resp = Resp::err(400, &self.to_message());
                HttpResponse::BadRequest().json(resp)
            }
            BusinessError::Unauthorized => {
                let resp = Resp::err(401, &self.to_message());
                HttpResponse::Unauthorized().json(resp)
            }
        }
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
    message: String,
    // 页数
    page: Option<i64>,
    // 每页最大值
    page_size: Option<i64>,
    // 查询内容总数
    total: Option<i64>,
    code: i32,
    // 数据内容
    body: Option<T>,
}

impl<T: Serialize> Resp<T> {
    #[allow(dead_code)]
    #[inline]
    pub fn ok(
        body: Option<T>,
        message: &str,
        page: Option<i64>,
        page_size: Option<i64>,
        total: Option<i64>,
    ) -> Self {
        Resp {
            success: true,
            code: 200,
            page,
            page_size,
            total,
            message: message.to_owned(),
            body,
        }
    }

    #[allow(dead_code)]
    pub fn to_json_result(&self) -> std::result::Result<HttpResponse, BusinessError> {
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
            page: None,
            page_size: None,
            total: None,
            message: message.to_owned(),
            body: None,
        }
    }
}

#[async_trait::async_trait]
pub trait CursorAsVec {
    async fn as_vec(&mut self) -> Result<Vec<Document>, BusinessError>;
}

#[async_trait::async_trait]
impl CursorAsVec for Cursor {
    async fn as_vec(&mut self) -> Result<Vec<Document>, BusinessError> {
        let mut list = vec![];
        while let Some(result) = self.next().await {
            let mut data = doc! {};
            let d = result.unwrap();
            data.insert(
                "_id",
                ObjectId::to_string(d.clone().get_object_id("_id").unwrap()),
            );
            // 为了让 _id 排在最前面
            for k in d.clone().keys() {
                if !k.eq("_id") {
                    data.insert(k, d.get(k).unwrap());
                }
            }
            list.push(data);
        }
        Ok(list)
    }
}

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
pub fn struct_to_document<'a, T: Sized + Serialize + Deserialize<'a>>(t: &T) -> Option<Document> {
    let mid: Option<Document> = bson::to_bson(t)
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

/// 处理文档 objectid
#[inline]
pub fn document_handle_id(doc: Document, ids: Option<Vec<&str>>) -> Option<Document> {
    let mut data = doc! {};
    let keys = doc.keys();
    let handle_id = vec!["_id", "create_by", "update_by"];
    // if !ids.is_none() {
    let handle_id = [handle_id, ids.unwrap_or_else(|| vec![])].concat();
    // };
    for k in keys {
        if handle_id.contains(&k.as_str()) {
            let oid = match doc.get_object_id(k) {
                Ok(id) => id.to_hex(),
                Err(_) => doc
                    .get(k)
                    .unwrap_or(&bson::Bson::Null)
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            };
            if !oid.is_empty() {
                data.insert(k, oid);
            } else {
                data.insert(k, &bson::Bson::Null);
            }
        } else {
            data.insert(k, doc.get(k).unwrap());
        }
        // if !k.eq("_id") {
        //     data.insert(k, doc.get(k).unwrap());
        // }
    }
    Some(data)
}
