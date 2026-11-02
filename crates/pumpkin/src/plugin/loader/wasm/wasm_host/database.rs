//! Database connections opened by the server on behalf of a plugin.

use std::{collections::HashMap, sync::Arc, time::Duration};

use mongodb::bson::{Bson, Document};
use redis::{AsyncCommands, Value as RedisValue, aio::MultiplexedConnection};
use serde_json::Value as Json;
use sqlx::{
    Column, Row, TypeInfo, ValueRef,
    mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlRow},
    postgres::{PgConnectOptions, PgPoolOptions, PgRow},
    {MySqlPool, PgPool},
};
use tokio::sync::Mutex;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONNECTIONS: u32 = 4;
const MAX_HANDLES: usize = 32;

/// Which engine a connection talks to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mysql,
    Postgres,
    Mongodb,
    Redis,
}

/// Connection parameters, each one given on its own.
pub struct Config {
    pub kind: Kind,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub auth_database: String,
    pub tls: bool,
}

/// A value exchanged with a database.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

#[derive(Default)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub affected: u64,
}

pub struct Statement {
    pub sql: String,
    pub params: Vec<Value>,
}

enum Connection {
    Mysql(MySqlPool),
    Postgres(PgPool),
    Mongo(mongodb::Database),
    Redis(MultiplexedConnection),
}

/// The connections of one plugin. They close when the manager is dropped.
#[derive(Default)]
pub struct DatabaseManager {
    next: std::sync::atomic::AtomicU32,
    connections: Mutex<HashMap<u32, Arc<Connection>>>,
}

impl DatabaseManager {
    pub async fn open(&self, config: Config) -> Result<u32, String> {
        if self.connections.lock().await.len() >= MAX_HANDLES {
            return Err("too many open database connections".into());
        }
        let connection = tokio::time::timeout(CONNECT_TIMEOUT, connect(&config))
            .await
            .map_err(|_| "the database did not answer in time".to_string())??;
        let handle = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        self.connections
            .lock()
            .await
            .insert(handle, Arc::new(connection));
        Ok(handle)
    }

    pub async fn close(&self, handle: u32) {
        self.connections.lock().await.remove(&handle);
    }

    async fn get(&self, handle: u32) -> Result<Arc<Connection>, String> {
        self.connections
            .lock()
            .await
            .get(&handle)
            .cloned()
            .ok_or_else(|| "unknown database handle".to_string())
    }

    pub async fn sql_run(&self, handle: u32, statement: Statement) -> Result<SqlResult, String> {
        match &*self.get(handle).await? {
            Connection::Mysql(pool) => mysql_run(pool, &statement).await,
            Connection::Postgres(pool) => pg_run(pool, &statement).await,
            _ => Err("this connection does not speak SQL".into()),
        }
    }

    pub async fn sql_transaction(
        &self,
        handle: u32,
        statements: Vec<Statement>,
    ) -> Result<(), String> {
        match &*self.get(handle).await? {
            Connection::Mysql(pool) => {
                let mut tx = pool.begin().await.map_err(err)?;
                for statement in &statements {
                    let mut query = sqlx::query(&statement.sql);
                    for param in &statement.params {
                        query = bind_mysql(query, param);
                    }
                    query.execute(&mut *tx).await.map_err(err)?;
                }
                tx.commit().await.map_err(err)
            }
            Connection::Postgres(pool) => {
                let mut tx = pool.begin().await.map_err(err)?;
                for statement in &statements {
                    let mut query = sqlx::query(&statement.sql);
                    for param in &statement.params {
                        query = bind_pg(query, param);
                    }
                    query.execute(&mut *tx).await.map_err(err)?;
                }
                tx.commit().await.map_err(err)
            }
            _ => Err("this connection does not speak SQL".into()),
        }
    }

    pub async fn mongo_command(&self, handle: u32, command: &str) -> Result<String, String> {
        let Connection::Mongo(database) = &*self.get(handle).await? else {
            return Err("this connection is not MongoDB".into());
        };
        let json: Json = serde_json::from_str(command).map_err(err)?;
        let Bson::Document(document) = Bson::try_from(json).map_err(err)? else {
            return Err("a MongoDB command must be a JSON object".into());
        };
        let reply: Document = database
            .run_command(command_first(document))
            .await
            .map_err(err)?;
        serde_json::to_string(&Bson::Document(reply).into_relaxed_extjson()).map_err(err)
    }

    pub async fn redis_command(
        &self,
        handle: u32,
        args: Vec<Vec<u8>>,
    ) -> Result<Vec<Value>, String> {
        let Connection::Redis(connection) = &*self.get(handle).await? else {
            return Err("this connection is not Redis".into());
        };
        let mut args = args.into_iter();
        let Some(name) = args.next() else {
            return Err("empty Redis command".into());
        };
        let mut command = redis::cmd(&String::from_utf8_lossy(&name));
        for arg in args {
            command.arg(arg);
        }
        let mut connection = connection.clone();
        let reply: RedisValue = command.query_async(&mut connection).await.map_err(err)?;
        Ok(flatten_redis(reply))
    }
}

/// Commands a plugin may run. JSON objects lose their key order on the way in, but MongoDB
/// reads the first key of a command document as the command name, so it is put back first.
const MONGO_COMMANDS: &[&str] = &[
    "find", "update", "insert", "delete", "findAndModify", "aggregate", "count", "distinct",
    "drop", "create", "createIndexes", "dropIndexes", "listCollections", "listIndexes", "ping",
    "hello", "isMaster", "buildInfo", "getMore", "killCursors", "dropDatabase", "collStats",
];

fn command_first(document: Document) -> Document {
    let Some(name) = document
        .keys()
        .find(|key| MONGO_COMMANDS.contains(&key.as_str()))
        .cloned()
    else {
        return document;
    };
    let mut ordered = Document::new();
    if let Some(value) = document.get(&name) {
        ordered.insert(name.clone(), value.clone());
    }
    for (key, value) in document {
        if key != name {
            ordered.insert(key, value);
        }
    }
    ordered
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

async fn connect(config: &Config) -> Result<Connection, String> {
    match config.kind {
        Kind::Mysql => {
            let mut options = MySqlConnectOptions::new()
                .host(&config.host)
                .port(config.port)
                .username(&config.username)
                .password(&config.password)
                .ssl_mode(if config.tls {
                    sqlx::mysql::MySqlSslMode::Required
                } else {
                    sqlx::mysql::MySqlSslMode::Disabled
                });
            if !config.database.is_empty() {
                options = options.database(&config.database);
            }
            let pool = MySqlPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .acquire_timeout(CONNECT_TIMEOUT)
                .connect_with(options)
                .await
                .map_err(err)?;
            Ok(Connection::Mysql(pool))
        }
        Kind::Postgres => {
            let mut options = PgConnectOptions::new()
                .host(&config.host)
                .port(config.port)
                .username(&config.username)
                .password(&config.password)
                .ssl_mode(if config.tls {
                    sqlx::postgres::PgSslMode::Require
                } else {
                    sqlx::postgres::PgSslMode::Disable
                });
            if !config.database.is_empty() {
                options = options.database(&config.database);
            }
            let pool = PgPoolOptions::new()
                .max_connections(MAX_CONNECTIONS)
                .acquire_timeout(CONNECT_TIMEOUT)
                .connect_with(options)
                .await
                .map_err(err)?;
            Ok(Connection::Postgres(pool))
        }
        Kind::Mongodb => {
            let mut options = mongodb::options::ClientOptions::default();
            options.hosts = vec![mongodb::options::ServerAddress::Tcp {
                host: config.host.clone(),
                port: Some(config.port),
            }];
            options.connect_timeout = Some(CONNECT_TIMEOUT);
            options.server_selection_timeout = Some(CONNECT_TIMEOUT);
            options.max_pool_size = Some(MAX_CONNECTIONS);
            if !config.username.is_empty() {
                let mut credential = mongodb::options::Credential::default();
                credential.username = Some(config.username.clone());
                credential.password = Some(config.password.clone());
                credential.source = Some(if config.auth_database.is_empty() {
                    "admin".into()
                } else {
                    config.auth_database.clone()
                });
                options.credential = Some(credential);
            }
            options.tls = config
                .tls
                .then(|| mongodb::options::Tls::Enabled(mongodb::options::TlsOptions::default()));
            let client = mongodb::Client::with_options(options).map_err(err)?;
            let database = client.database(if config.database.is_empty() {
                "admin"
            } else {
                &config.database
            });
            database
                .run_command(mongodb::bson::doc! { "ping": 1 })
                .await
                .map_err(err)?;
            Ok(Connection::Mongo(database))
        }
        Kind::Redis => {
            let info = redis::ConnectionInfo {
                addr: if config.tls {
                    redis::ConnectionAddr::TcpTls {
                        host: config.host.clone(),
                        port: config.port,
                        insecure: false,
                        tls_params: None,
                    }
                } else {
                    redis::ConnectionAddr::Tcp(config.host.clone(), config.port)
                },
                redis: redis::RedisConnectionInfo {
                    db: config.database.trim().parse().unwrap_or(0),
                    username: (!config.username.is_empty()).then(|| config.username.clone()),
                    password: (!config.password.is_empty()).then(|| config.password.clone()),
                    ..Default::default()
                },
            };
            let client = redis::Client::open(info).map_err(err)?;
            let mut connection = client
                .get_multiplexed_async_connection()
                .await
                .map_err(err)?;
            let _: String = connection.ping().await.map_err(err)?;
            Ok(Connection::Redis(connection))
        }
    }
}

fn bind_mysql<'q>(
    query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    value: &'q Value,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match value {
        Value::Null => query.bind(None::<String>),
        Value::Boolean(v) => query.bind(*v),
        Value::Integer(v) => query.bind(*v),
        Value::Real(v) => query.bind(*v),
        Value::Text(v) => query.bind(v.as_str()),
        Value::Blob(v) => query.bind(v.as_slice()),
    }
}

fn bind_pg<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    value: &'q Value,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match value {
        Value::Null => query.bind(None::<String>),
        Value::Boolean(v) => query.bind(*v),
        Value::Integer(v) => query.bind(*v),
        Value::Real(v) => query.bind(*v),
        Value::Text(v) => query.bind(v.as_str()),
        Value::Blob(v) => query.bind(v.as_slice()),
    }
}

fn returns_rows(sql: &str) -> bool {
    let sql = sql.trim_start().to_ascii_lowercase();
    ["select", "show", "with", "explain", "describe", "values"]
        .iter()
        .any(|keyword| sql.starts_with(keyword))
        || sql.contains(" returning ")
}

async fn mysql_run(pool: &MySqlPool, statement: &Statement) -> Result<SqlResult, String> {
    let mut query = sqlx::query(&statement.sql);
    for param in &statement.params {
        query = bind_mysql(query, param);
    }
    if !returns_rows(&statement.sql) {
        let done = query.execute(pool).await.map_err(err)?;
        return Ok(SqlResult {
            affected: done.rows_affected(),
            ..SqlResult::default()
        });
    }
    let rows = query.fetch_all(pool).await.map_err(err)?;
    Ok(SqlResult {
        columns: rows
            .first()
            .map(|row| row.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default(),
        affected: 0,
        rows: rows.iter().map(mysql_row).collect(),
    })
}

async fn pg_run(pool: &PgPool, statement: &Statement) -> Result<SqlResult, String> {
    let mut query = sqlx::query(&statement.sql);
    for param in &statement.params {
        query = bind_pg(query, param);
    }
    if !returns_rows(&statement.sql) {
        let done = query.execute(pool).await.map_err(err)?;
        return Ok(SqlResult {
            affected: done.rows_affected(),
            ..SqlResult::default()
        });
    }
    let rows = query.fetch_all(pool).await.map_err(err)?;
    Ok(SqlResult {
        columns: rows
            .first()
            .map(|row| row.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default(),
        affected: 0,
        rows: rows.iter().map(pg_row).collect(),
    })
}

fn mysql_row(row: &MySqlRow) -> Vec<Value> {
    (0..row.columns().len())
        .map(|index| {
            let raw = row.try_get_raw(index);
            if raw.as_ref().map_or(true, |raw| raw.is_null()) {
                return Value::Null;
            }
            let name = row.columns()[index].type_info().name().to_ascii_uppercase();
            match name.as_str() {
                "BOOLEAN" => row.try_get::<bool, _>(index).map(Value::Boolean),
                "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT" | "BIGINT" | "YEAR" => {
                    row.try_get::<i64, _>(index).map(Value::Integer)
                }
                "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "INT UNSIGNED"
                | "MEDIUMINT UNSIGNED" | "BIGINT UNSIGNED" => row
                    .try_get::<u64, _>(index)
                    .map(|v| Value::Integer(i64::try_from(v).unwrap_or(i64::MAX))),
                "FLOAT" | "DOUBLE" => row.try_get::<f64, _>(index).map(Value::Real),
                "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "VARBINARY" | "BINARY" => {
                    row.try_get::<Vec<u8>, _>(index).map(Value::Blob)
                }
                _ => row.try_get::<String, _>(index).map(Value::Text),
            }
            .unwrap_or(Value::Null)
        })
        .collect()
}

fn pg_row(row: &PgRow) -> Vec<Value> {
    (0..row.columns().len())
        .map(|index| {
            let raw = row.try_get_raw(index);
            if raw.as_ref().map_or(true, |raw| raw.is_null()) {
                return Value::Null;
            }
            let name = row.columns()[index].type_info().name().to_ascii_uppercase();
            match name.as_str() {
                "BOOL" => row.try_get::<bool, _>(index).map(Value::Boolean),
                "INT2" => row.try_get::<i16, _>(index).map(|v| Value::Integer(i64::from(v))),
                "INT4" => row.try_get::<i32, _>(index).map(|v| Value::Integer(i64::from(v))),
                "INT8" => row.try_get::<i64, _>(index).map(Value::Integer),
                "FLOAT4" => row.try_get::<f32, _>(index).map(|v| Value::Real(f64::from(v))),
                "FLOAT8" => row.try_get::<f64, _>(index).map(Value::Real),
                "BYTEA" => row.try_get::<Vec<u8>, _>(index).map(Value::Blob),
                _ => row.try_get::<String, _>(index).map(Value::Text),
            }
            .unwrap_or(Value::Null)
        })
        .collect()
}

fn flatten_redis(value: RedisValue) -> Vec<Value> {
    match value {
        RedisValue::Array(items) | RedisValue::Set(items) => {
            items.into_iter().map(single_redis).collect()
        }
        other => vec![single_redis(other)],
    }
}

fn single_redis(value: RedisValue) -> Value {
    match value {
        RedisValue::Nil => Value::Null,
        RedisValue::Int(v) => Value::Integer(v),
        RedisValue::Boolean(v) => Value::Boolean(v),
        RedisValue::Double(v) => Value::Real(v),
        RedisValue::SimpleString(v) => Value::Text(v),
        RedisValue::Okay => Value::Text("OK".into()),
        RedisValue::BulkString(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Value::Text(text),
            Err(error) => Value::Blob(error.into_bytes()),
        },
        _ => Value::Null,
    }
}
