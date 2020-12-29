use super::*;
use crate::utils::*;
use bson::{oid::ObjectId, Document};
use core::fmt;
use futures::StreamExt;
use mongodb::{
    bson::doc,
    options::{
        ClientOptions, CountOptions, FindOneAndUpdateOptions, FindOneOptions, FindOptions,
        ReturnDocument,
    },
    Client, Collection, Cursor, Database,
};
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::sync::Mutex;
use std::time::Duration;

lazy_static! {
    static ref DB: Mutex<Option<Database>> = Mutex::new(None);
}

pub async fn init(uri: &str, db: &str) {
    let mut options = ClientOptions::parse(uri).await.unwrap();
    options.connect_timeout = Some(Duration::from_secs(3));
    options.heartbeat_freq = Some(Duration::from_secs(3));
    options.server_selection_timeout = Some(Duration::from_secs(3));
    let client = Client::with_options(options).unwrap();
    let mut lock = DB.lock().unwrap();
    *lock = Some(client.database(db));
}

pub fn collection(name: &str) -> Collection {
    let db = DB.lock().unwrap();
    (*db).as_ref().unwrap().collection(name)
}

pub struct Dao {
    pub coll: Collection,
}

impl Dao {
    pub fn new(name: &str) -> Self {
        let coll = collection(name);
        Dao { coll }
    }

    /// 保存
    pub async fn save<T>(&self, data: &T) -> Result<ObjectId, BusinessError>
    where
        T: Serialize,
    {
        let mut doc = bson::to_bson(data)
            .unwrap()
            .as_document()
            .unwrap()
            .to_owned();
        doc.insert("create_time", date_time::timestamp());
        doc.insert("update_time", date_time::timestamp());
        doc.insert("_id", ObjectId::new());
        let ret = self.coll.insert_one(doc, None).await?;
        let oid = ret
            .inserted_id
            .as_object_id()
            .expect("Retrieved _id should have been of type ObjectId");
        Ok(oid.to_owned())
    }

    /// 根据id 查询一条
    pub async fn find_by_id(&self, id: ObjectId) -> Result<Option<Document>, BusinessError> {
        let filter = doc! {"_id": id};
        let mut opt = FindOneOptions::default();
        opt.max_time = Some(Duration::from_secs(3));
        let data = self.coll.find_one(filter, opt).await.unwrap();

        match data {
            Some(d) => {
                // let data: T = bson::from_document(d)
                //     .map_err(|e| BusinessError::InternalError { source: anyhow!(e) })
                //     .unwrap();
                let data = document_handle_id(d).unwrap();
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// 根据条件查询一条
    pub async fn find_one(&self, filter: Document) -> Result<Option<Document>, BusinessError> {
        let mut opt = FindOneOptions::default();
        opt.max_time = Some(Duration::from_secs(3));
        let data = self.coll.find_one(filter, opt).await.unwrap();

        match data {
            Some(d) => {
                let data = document_handle_id(d).unwrap();
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// 查询
    pub async fn find(
        &self,
        filter: Document,
        limit: i64,
        page: i64,
        sort_name: &str,
        sort_order: &str,
    ) -> Result<Vec<Document>, BusinessError> {
        let mut opt = FindOptions::default();
        // 限制条数
        opt.limit = Some(limit);

        // 设置一页多少条
        let skip = limit * (page - 1);
        opt.skip = Some(skip);

        // 设置查询排序  默认创建时间的倒序
        let mut sort = doc! {};
        if !sort_name.is_empty() && !sort_order.is_empty() {
            if sort_order.eq("desc") {
                sort.insert(sort_name, -1);
            } else {
                sort.insert(sort_name, 1);
            }
        } else {
            sort.insert("create_time", -1);
        }

        opt.sort = Some(sort);
        let mut cursor = self.coll.find(Some(filter), opt).await.unwrap();
        let list = cursor.as_vec().await;
        match list {
            Ok(list) => Ok(list),
            Err(e) => Err(BusinessError::InternalError { source: anyhow!(e) })?,
        }
    }

    /// 获取查询总数
    pub async fn count(&self, filter: Document) -> Result<i64, BusinessError> {
        let opt = CountOptions::default();
        let count = self.coll.count_documents(Some(filter), opt).await;
        match count {
            Ok(count) => Ok(count),
            Err(e) => {
                return Err(BusinessError::InternalError { source: anyhow!(e) })?;
            }
        }
    }

    /// 更新数据
    pub async fn update(
        &self,
        id: ObjectId,
        data: Document,
    ) -> Result<Option<Document>, BusinessError> {
        let filter = doc! {"_id":id};
        let mut doc = data;
        doc.insert("update_time", date_time::timestamp());
        doc.remove("_id");

        // 删除不需要的key
        let keys = doc.keys();
        let rm: Vec<String> = keys
            .filter(|k| doc.is_null(k))
            .map(|x| x.to_owned())
            .collect();
        for x in rm {
            doc.remove(&x);
        }
        let doc = doc! {"$set": doc};
        let mut opt = FindOneAndUpdateOptions::default();
        opt.return_document = Some(ReturnDocument::After);
        let data = match self.coll.find_one_and_update(filter, doc, opt).await {
            Ok(d) => d,
            Err(_) => {
                return Err(BusinessError::InternalError {
                    source: anyhow!("修改失败"),
                })
            }
        };

        match data {
            Some(d) => {
                // let data: T = bson::from_document(d)
                //     .map_err(|e| BusinessError::InternalError { source: anyhow!(e) })
                //     .unwrap();
                // Ok(Some(data))
                let data = document_handle_id(d).unwrap();
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// 删除
    pub async fn remove(&self, ids: String) -> Result<i64, BusinessError> {
        let arr: Vec<&str> = ids.rsplit(",").collect();
        let mut remids: Vec<ObjectId> = Vec::new();
        for id in arr.iter() {
            let oid = match ObjectId::with_string(*id) {
                Ok(oid) => oid,
                Err(_) => {
                    return Err(BusinessError::InternalError {
                        source: anyhow!("_id 字段错误"),
                    })
                }
            };
            remids.push(oid)
        }
        let mut doc: Document = doc! {};
        doc.insert("$in", remids);
        let d = doc! {"_id":doc};
        let result = self.coll.delete_many(d, None).await;
        match result {
            Ok(res) => Ok(res.deleted_count),
            Err(_) => {
                return Err(BusinessError::InternalError {
                    source: anyhow!("删除失败"),
                })?
            }
        }
    }
}
