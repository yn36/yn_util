use super::*;
use crate::utils::*;
use bson::{oid::ObjectId, Document};
use mongodb::{
    bson::doc,
    options::{
        ClientOptions, CountOptions, FindOneAndUpdateOptions, FindOneOptions, FindOptions,
        ReturnDocument,
    },
    Client, Collection, Database,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

lazy_static! {
    // 单个数据库
    static ref DB: Mutex<Option<Database>> = Mutex::new(None);
    // 数据库集合  多个数据库
    static ref DBS:Arc<HashMap<String,Mutex<Option<Database>>>> = {
        let mut map = HashMap::new();
        map.insert("testDB".to_string(), Mutex::new(None));
        // 地区省份数据库
        map.insert("position".to_string(), Mutex::new(None));

        Arc::new(map)
    };
}

/// 初始化单数据库
pub async fn init(uri: &str, db: &str) {
    let mut options = ClientOptions::parse(uri).await.unwrap();
    options.connect_timeout = Some(Duration::from_secs(3));
    options.heartbeat_freq = Some(Duration::from_secs(3));
    options.server_selection_timeout = Some(Duration::from_secs(3));
    let client = Client::with_options(options).unwrap();
    let mut lock = DB.lock().unwrap();
    *lock = Some(client.database(db));
}

/// 初始化多数据库集合
pub async fn init_dbs(uri: &str) {
    let mut options = ClientOptions::parse(uri).await.unwrap();
    options.connect_timeout = Some(Duration::from_secs(3));
    options.heartbeat_freq = Some(Duration::from_secs(3));
    options.server_selection_timeout = Some(Duration::from_secs(3));
    let client = Client::with_options(options).unwrap();
    let keys = DBS.keys();

    for key in keys.into_iter() {
        if let Some(item) = DBS.get(key) {
            if let Ok(mut lock) = item.lock() {
                info!("{},数据库连接成功", key);
                *lock = Some(client.database(key));
            }
        }
    }
}

pub fn collection(db_name: &str, name: &str) -> Collection {
    let db = DBS.get(db_name).unwrap().lock().unwrap();
    (*db).as_ref().unwrap().collection(name)
}

pub struct Dao {
    pub coll: Collection,
}

impl Dao {
    pub fn new(db_name: &str, name: &str) -> Self {
        let coll = collection(db_name, name);
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
        doc.insert("create_time", date_time::to_string());
        doc.insert("update_time", date_time::to_string());
        doc.insert("_id", ObjectId::new());
        let ret = self.coll.insert_one(doc, None).await?;
        let oid = ret
            .inserted_id
            .as_object_id()
            .expect("Retrieved _id should have been of type ObjectId");
        Ok(oid.to_owned())
    }

    /// 保存多条数据
    pub async fn save_many(
        &self,
        datas: impl IntoIterator<Item = Document>,
    ) -> Result<mongodb::results::InsertManyResult, BusinessError> {
        let mut docs = vec![];

        for mut doc in datas {
            doc.insert("create_time", date_time::to_string());
            doc.insert("update_time", date_time::to_string());
            doc.insert("_id", ObjectId::new());
            docs.push(doc)
        }

        // let ret = self.coll.insert_many(docs, None).await;

        // let docs = vec![
        //     doc! { "title": "1984", "author": "George Orwell" },
        //     doc! { "title": "Animal Farm", "author": "George Orwell" },
        //     doc! { "title": "The Great Gatsby", "author": "F. Scott Fitzgerald" },
        // ];
        let ret = self.coll.insert_many(docs, None).await;
        match ret {
            Ok(value) => Ok(value),
            Err(e) => return Err(BusinessError::InternalError { source: anyhow!(e) }),
        }
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
                let data = document_handle_id(d, None).unwrap();
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
                let data = document_handle_id(d, None).unwrap();
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// 查询
    /// oid_type - objectid类型
    pub async fn find(
        &self,
        filter: Document,
        limit: Option<i64>,
        page: Option<i64>,
        sort_name: Option<String>,
        sort_order: Option<String>,
        is_all: bool,
        oid_type: Option<Vec<&str>>,
    ) -> Result<Vec<Document>, BusinessError> {
        let mut opt = FindOptions::default();
        if !is_all {
            // 限制条数
            opt.limit = Some(limit.unwrap_or(10));
        }

        // 设置一页多少条
        let skip = limit.unwrap_or(10) * (page.unwrap_or(1) - 1);
        opt.skip = Some(skip);

        // 设置查询排序  默认创建时间的倒序
        let mut sort = doc! {};
        if !sort_name.clone().unwrap_or("".to_string()).is_empty()
            && !sort_order.clone().unwrap_or("".to_string()).is_empty()
        {
            if sort_order.unwrap().eq("desc") {
                sort.insert(sort_name.unwrap(), -1);
            } else {
                // sort.insert("province_code", 1);
                sort.insert(sort_name.unwrap(), 1);
            }
        } else {
            sort.insert("create_time", -1);
        }

        opt.sort = Some(sort);

        // 模糊查询
        let keys = filter.keys();
        let mut d = doc! {};
        let mut list = vec![];

        let hoids = vec!["_id"];

        let hoids = [hoids, oid_type.unwrap_or_else(|| vec![])].concat();

        for k in keys.into_iter() {
            info!(
                "-------k = {:?} -- -hoids = {:?} --- !hoids.contains(&k) = {:?}",
                k,
                hoids,
                !hoids.contains(&k.as_str())
            );
            if !hoids.contains(&k.as_str()) {
                match filter.get(k).unwrap().as_str() {
                    Some(_) => {
                        let doc = doc! { k: bson::Regex {pattern:filter.get(k).unwrap().as_str().unwrap().to_string(),options:"i".to_string()}}.into();
                        list.push(doc);
                    }
                    None => {
                        let doc =
                            doc! { k:bson::Bson::Int32(filter.get(k).unwrap().as_i32().unwrap())}
                                .into();
                        list.push(doc);
                    }
                }
            } else {
                let oid = filter.get(k).unwrap().as_str().unwrap();
                let oid = ObjectId::with_string(oid).unwrap();
                d.insert(k, oid);
            }
        }
        if list.len() > 0 {
            d.insert("$and", bson::Bson::Array(list));
        }
        info!("d = {:?}", d);
        let mut cursor = self.coll.find(Some(d), opt).await.unwrap();
        let list = cursor.as_vec().await;
        match list {
            Ok(list) => {
                let mut v = vec![];
                for item in list {
                    v.push(document_handle_id(item, None).unwrap())
                }
                Ok(v)
            }
            Err(e) => Err(BusinessError::InternalError { source: anyhow!(e) })?,
        }
    }

    /// 获取查询总数
    pub async fn count(&self, filter: Document) -> Result<i64, BusinessError> {
        let opt = CountOptions::default();
        // 模糊查询
        let keys = filter.keys();
        let mut d = doc! {};
        let mut list = vec![];
        for k in keys.into_iter() {
            if !k.eq("_id") {
                match filter.get(k).unwrap().as_str() {
                    Some(_) => {
                        let doc = doc! { k: bson::Regex {pattern:filter.get(k).unwrap().as_str().unwrap().to_string(),options:"i".to_string()}}.into();
                        list.push(doc);
                    }
                    None => {
                        let doc =
                            doc! { k:bson::Bson::Int32(filter.get(k).unwrap().as_i32().unwrap())}
                                .into();
                        list.push(doc);
                    }
                }
            } else {
                let oid = filter.get("_id").unwrap().as_str().unwrap();
                let oid = ObjectId::with_string(oid).unwrap();
                d.insert("_id", oid);
            }
        }
        if list.len() > 0 {
            d.insert("$and", bson::Bson::Array(list));
        }
        let count = self.coll.count_documents(Some(d), opt).await;
        match count {
            Ok(count) => Ok(count),
            Err(e) => {
                return Err(BusinessError::InternalError { source: anyhow!(e) })?;
            }
        }
    }

    /// 更新数据
    pub async fn update(&self, data: Document) -> Result<Option<Document>, BusinessError> {
        // let oid = match data.get_object_id("_id") {
        //     Ok(id) => id.to_hex(),
        //     Err(_) => data.get("_id").unwrap().as_str().unwrap(),
        // };
        let oid = data.get("_id").unwrap().as_str().unwrap();
        let oid = bson::oid::ObjectId::with_string(oid).unwrap();
        let filter = doc! {"_id":oid};

        let mut doc = data;
        doc.insert("update_time", date_time::to_string());
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
                let data = document_handle_id(d, None).unwrap();
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
            Ok(res) => {
                if res.deleted_count > 0 {
                    Ok(res.deleted_count)
                } else {
                    return Err(BusinessError::InternalError {
                        source: anyhow!("删除失败,请提供正确的id"),
                    });
                }
            }
            Err(_) => {
                return Err(BusinessError::InternalError {
                    source: anyhow!("删除失败"),
                })?
            }
        }
    }
}
