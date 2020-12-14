use super::*;
use mongodb::{self, Client, Collection};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Mongodb {
    pub host: String,
    pub user: String,
    pub password: String,
    pub port: usize,
}

impl Mongodb {
    pub fn new(db: Mongodb) -> Self {
        Mongodb {
            host: db.host,
            user: db.user,
            password: db.password,
            port: db.port,
        }
    }

    /// 初始化 连接数据库
    pub fn create_mongo_client(mongo: Mongodb) -> Client {
        // let mongo: Mongodb = Mongodb::default();
        let conn_url: String = format!(
            "mongodb://{}:{}@{}:{}",
            mongo.user, mongo.password, mongo.host, mongo.port
        );
        info!("数据库建立连接,{}", &conn_url);
        Client::with_uri_str(&conn_url)
            .ok()
            .expect("数据库连接失败")
    }

    /// 获取数据库连接
    pub fn collection(MONGO: Client, data_base: String, coll_name: String) -> Collection {
        MONGO.database(&data_base).collection(coll_name.as_str())
    }
}

/// 初始化数据库
pub fn create_mongo_client(conn_url: String) -> Client {
    info!("数据库建立链接,{}", &conn_url);
    Client::with_uri_str(&conn_url)
        .ok()
        .expect("数据库链接失败")
}
