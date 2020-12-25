use super::*;
use crate::utils::*;
use bson::{oid::ObjectId, Document};
use core::fmt;
use futures::StreamExt;
use mongodb::{
    bson::doc,
    options::{ClientOptions, CountOptions, FindOneOptions, FindOptions},
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
    coll: Collection,
}

impl Dao {
    pub fn new(name: &str) -> Self {
        let coll = collection(name);
        Dao { coll }
    }

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
        info!("doc = {:#?}", doc);
        let ret = self.coll.insert_one(doc, None).await?;
        let oid = ret
            .inserted_id
            .as_object_id()
            .expect("Retrieved _id should have been of type ObjectId");
        Ok(oid.to_owned())
    }

    pub async fn find_by_id<T>(&self, id: ObjectId) -> Result<Option<T>, BusinessError>
    where
        T: DeserializeOwned,
    {
        let filter = doc! {"_id": id};
        let mut opt = FindOneOptions::default();
        opt.max_time = Some(Duration::from_secs(3));
        let data = self.coll.find_one(filter, opt).await.unwrap();

        match data {
            Some(d) => {
                let data: T = bson::from_document(d)
                    .map_err(|e| BusinessError::InternalError { source: anyhow!(e) })
                    .unwrap();
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    pub async fn find<T>(
        &self,
        filter: Document,
        limit: i64,
        page: i64,
        sort_name: &str,
        sort_order: &str,
    ) -> Result<Vec<T>, BusinessError>
    where
        T: DeserializeOwned + Send,
    {
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
}
