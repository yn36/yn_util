use super::*;
use mongodb::{self, options::ClientOptions, Client, Collection};

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
    pub async fn create_mongo_client(mongo: Mongodb) -> Client {
        // let mongo: Mongodb = Mongodb::default();
        let conn_url: String = format!(
            "mongodb://{}:{}@{}:{}",
            mongo.user, mongo.password, mongo.host, mongo.port
        );
        info!("数据库建立连接,{}", &conn_url);
        let client_options = ClientOptions::parse(&conn_url).await.unwrap();
        Client::with_options(client_options).unwrap()
    }

    /// 获取数据库连接
    pub fn collection(mongo: Client, data_base: String, coll_name: String) -> Collection {
        mongo.database(&data_base).collection(coll_name.as_str())
    }
}

/// 初始化数据库
pub async fn create_mongo_client(conn_url: String) -> Client {
    info!("数据库建立链接,{}", &conn_url);
    let client_options = ClientOptions::parse(&conn_url).await.unwrap();
    Client::with_options(client_options).unwrap()
}
