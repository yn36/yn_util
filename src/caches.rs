extern crate redis;
use super::*;
use lazy_static::*;
use redis::cluster::{ClusterClient, ClusterConnection};
use redis::Commands;
use redis::{Client, Connection};
use std::sync::Mutex;

lazy_static! {
    static ref CLUSTERCACHES: Mutex<Vec<ClusterClient>> = Mutex::new(vec![]);
    static ref CACHES: Mutex<Vec<Client>> = Mutex::new(vec![]);
}

/// 初始化缓存数据库连接 当前只支持 redis
///
/// # ! `集群使用`
/// [`conn_struct`]:Document
/// # Example
/// ``` rust
/// let nodes = vec!["redis://127.0.0.1/"];
/// init_cluster_connections(nodes);
/// ```
pub fn init_cluster_connections(nodes: Vec<String>) {
    let client = ClusterClient::open(nodes).unwrap();
    let mut pools = CLUSTERCACHES.lock().unwrap();
    (*pools).push(client);
}

/// 獲取資料庫連接
///
/// # ! `集群使用`
/// ```rust
/// let mut redis = cache::get_cluster_conn();
/// let _val = redis.get::<&str, String>("hello").unwrap_or("world".to_owned());
/// ```
pub fn get_cluster_conn() -> ClusterConnection {
    let pools = CLUSTERCACHES.lock().unwrap();

    unsafe { (*pools).get_unchecked(0).get_connection().unwrap() }
}

/// redis 测试
pub fn set() {
    let _: () = get_conn().set("test", "value").unwrap();
    let rv: String = get_conn().get("test").unwrap();
    info!("rv = {:?}", rv);
}

/// 初始化資料庫連接
///
/// # ! `单体使用`
/// 連接字符串形似: 'redis://127.0.0.1'
pub fn init_connections(conn_string: &str) {
    let cache = redis::Client::open(conn_string).unwrap();
    let mut pools = CACHES.lock().unwrap();
    (*pools).push(cache);
}

/// 獲取資料庫連接
///
/// # ! `单体使用`
/// ```rust
/// let mut redis = cache::get_conn();
/// let _val = redis.get::<&str, String>("hello").unwrap_or("world".to_owned());
/// ```
pub fn get_conn() -> Connection {
    let pools = CACHES.lock().unwrap();
    unsafe { (*pools).get_unchecked(0).get_connection().unwrap() }
}
