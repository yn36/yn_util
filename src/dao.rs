use super::*;
use crate::utils::{BusinessError, Result};
use bson::{oid::ObjectId, Document};
use core::fmt;
use futures::StreamExt;
use mongodb::{
    bson::doc,
    options::{ClientOptions, CountOptions, FindOneOptions, FindOptions},
    Client, Collection, Cursor, Database,
};
use serde::{de, de::DeserializeOwned, Deserializer, Serializer};
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

/// 序列化id
pub fn serialize_object_id<S>(oid: &Option<ObjectId>, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match oid.as_ref().map(|x| x.to_hex()) {
        Some(v) => s.serialize_str(&v),
        None => s.serialize_none(),
    }
}

/// 反序列化id
#[allow(dead_code)]
pub fn deserialize_object_id<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<ObjectId>, D::Error>
where
    D: Deserializer<'de>,
{
    struct JsonOptionObjectIdVisitor;
    impl<'de> de::Visitor<'de> for JsonOptionObjectIdVisitor {
        type Value = Option<ObjectId>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an object id hash value")
        }

        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                return Ok(None);
            }
            Ok(ObjectId::with_string(v).ok())
        }
    }
    deserializer.deserialize_any(JsonOptionObjectIdVisitor)
}

pub struct Dao {
    coll: Collection,
}

impl Dao {
    pub fn new(name: &str) -> Self {
        let coll = collection(name);
        Dao { coll }
    }

    fn get_object_id(id: &str) -> Result<ObjectId> {
        ObjectId::with_string(id).map_err(|_| BusinessError::ValidationError {
            field: id.to_owned(),
        })
    }

    pub async fn save<T>(&self, data: &T) -> Result<ObjectId>
    where
        T: Serialize,
    {
        let doc = bson::to_bson(data)
            .unwrap()
            .as_document()
            .unwrap()
            .to_owned();
        let ret = self.coll.insert_one(doc, None).await?;
        let id = ret
            .inserted_id
            .as_object_id()
            .expect("Retrieved _id should have been of type ObjectId");
        Ok(id.to_owned())
    }

    pub async fn save_data<T>(&self, data: T) -> Result<MongoObject<T>>
    where
        T: Serialize,
    {
        let id = self.save(&data).await?;
        Ok(MongoObject { id: Some(id), data })
    }

    pub async fn delete(&self, id: &str) -> Result<i64> {
        let oid = Self::get_object_id(id)?;
        let filter = doc! {"_id": oid};
        let res = self.coll.delete_one(filter, None).await?;
        Ok(res.deleted_count)
    }

    pub async fn update<T>(&self, data: &MongoObject<T>) -> Result<()>
    where
        T: Serialize,
    {
        let filter = doc! {"_id":data.id.as_ref().unwrap()};
        let doc = bson::to_bson(&data.data)
            .unwrap()
            .as_document()
            .unwrap()
            .to_owned();
        self.coll.update_one(filter, doc, None).await?;
        Ok(())
    }

    pub async fn find_by_id<T>(&self, id: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let oid = Self::get_object_id(id)?;
        let filter = doc! {"_id": oid};
        let mut opt = FindOneOptions::default();
        opt.max_time = Some(Duration::from_secs(3));
        let data = self.coll.find_one(filter, opt).await?;

        match data {
            Some(d) => {
                let data: T = bson::from_document(d)
                    .map_err(|e| BusinessError::InternalError { source: anyhow!(e) })?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    pub async fn count(&self, filter: impl Into<Option<Document>>) -> Result<i64> {
        let opt = CountOptions::default();
        let count = self.coll.count_documents(filter, opt).await?;
        Ok(count)
    }

    pub async fn list<T>(
        &self,
        filter: impl Into<Option<Document>>,
        limit: i64,
        skip: i64,
    ) -> Result<Vec<MongoObject<T>>>
    where
        T: DeserializeOwned + Send,
    {
        let mut opt = FindOptions::default();
        opt.limit = Some(limit);
        opt.skip = Some(skip);

        let mut cursor = self.coll.find(filter, opt).await?;
        let list = cursor.as_vec().await?;
        Ok(list)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MongoObject<T> {
    #[serde(serialize_with = "serialize_object_id", rename(deserialize = "_id"))]
    pub id: Option<ObjectId>,
    #[serde(flatten)]
    pub data: T,
}

#[async_trait::async_trait]
pub trait CursorAsVec {
    async fn as_vec<T: DeserializeOwned + Send>(&mut self) -> Result<Vec<T>>;
}

#[async_trait::async_trait]
impl CursorAsVec for Cursor {
    async fn as_vec<T: DeserializeOwned + Send>(&mut self) -> Result<Vec<T>> {
        let mut list = vec![];
        while let Some(result) = self.next().await {
            let data = bson::from_document(result?)
                .map_err(|e| BusinessError::InternalError { source: anyhow!(e) })?;
            list.push(data);
        }
        Ok(list)
    }
}
