use crate::plugin::loader::wasm::wasm_host::wit::v0_1::pumpkin::plugin::database::{
    DbConfig, DbKind, DbValue, Host, SqlResult, SqlStatement,
};
use crate::plugin::{
    loader::wasm::wasm_host::{
        database::{self, Config, Kind, Statement, Value},
        state::PluginHostState,
    },
    permissions,
};

fn may_use_databases(state: &PluginHostState) -> bool {
    let has = |p: &str| state.permissions.iter().any(|granted| granted == p);
    has(permissions::NETWORK_DNS)
        && (has(permissions::NETWORK_TCP)
            || has(permissions::NETWORK_TCP_CONNECT)
            || has(permissions::NETWORK_OUTBOUND))
}

fn to_value(value: DbValue) -> Value {
    match value {
        DbValue::Null => Value::Null,
        DbValue::Boolean(v) => Value::Boolean(v),
        DbValue::Integer(v) => Value::Integer(v),
        DbValue::Real(v) => Value::Real(v),
        DbValue::Text(v) => Value::Text(v),
        DbValue::Blob(v) => Value::Blob(v),
    }
}

fn from_value(value: Value) -> DbValue {
    match value {
        Value::Null => DbValue::Null,
        Value::Boolean(v) => DbValue::Boolean(v),
        Value::Integer(v) => DbValue::Integer(v),
        Value::Real(v) => DbValue::Real(v),
        Value::Text(v) => DbValue::Text(v),
        Value::Blob(v) => DbValue::Blob(v),
    }
}

fn to_statement(statement: SqlStatement) -> Statement {
    Statement {
        sql: statement.sql,
        params: statement.params.into_iter().map(to_value).collect(),
    }
}

impl Host for PluginHostState {
    async fn open(&mut self, config: DbConfig) -> wasmtime::Result<Result<u32, String>> {
        if !may_use_databases(self) {
            return Ok(Err(
                "the plugin needs the network.tcp.connect and network.dns permissions".into(),
            ));
        }
        let kind = match config.kind {
            DbKind::Mysql | DbKind::Mariadb => Kind::Mysql,
            DbKind::Postgres => Kind::Postgres,
            DbKind::Mongodb => Kind::Mongodb,
            DbKind::Redis => Kind::Redis,
        };
        let databases = self.databases.clone();
        Ok(databases
            .open(Config {
                kind,
                host: config.host,
                port: config.port,
                database: config.database,
                username: config.username,
                password: config.password,
                auth_database: config.auth_database,
                tls: config.tls,
            })
            .await)
    }

    async fn close(&mut self, handle: u32) -> wasmtime::Result<()> {
        self.databases.clone().close(handle).await;
        Ok(())
    }

    async fn sql_run(
        &mut self,
        handle: u32,
        statement: SqlStatement,
    ) -> wasmtime::Result<Result<SqlResult, String>> {
        let databases = self.databases.clone();
        Ok(databases
            .sql_run(handle, to_statement(statement))
            .await
            .map(|result: database::SqlResult| SqlResult {
                columns: result.columns,
                rows: result
                    .rows
                    .into_iter()
                    .map(|row| row.into_iter().map(from_value).collect())
                    .collect(),
                affected: result.affected,
            }))
    }

    async fn sql_transaction(
        &mut self,
        handle: u32,
        statements: Vec<SqlStatement>,
    ) -> wasmtime::Result<Result<(), String>> {
        let databases = self.databases.clone();
        Ok(databases
            .sql_transaction(handle, statements.into_iter().map(to_statement).collect())
            .await)
    }

    async fn mongo_command(
        &mut self,
        handle: u32,
        command_json: String,
    ) -> wasmtime::Result<Result<String, String>> {
        let databases = self.databases.clone();
        Ok(databases.mongo_command(handle, &command_json).await)
    }

    async fn redis_command(
        &mut self,
        handle: u32,
        args: Vec<Vec<u8>>,
    ) -> wasmtime::Result<Result<Vec<DbValue>, String>> {
        let databases = self.databases.clone();
        Ok(databases
            .redis_command(handle, args)
            .await
            .map(|values| values.into_iter().map(from_value).collect()))
    }
}
