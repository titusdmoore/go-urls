use std::collections::BTreeMap;
use surrealdb::{
    sql::{Object, Value},
    Datastore, Response as SurrealResponse, Session,
};
pub type DB = (Datastore, Session);

pub async fn create_link((ds, ses): &DB, key: &str, url: &str) -> Result<String, SurrealError> {
    let sql = "CREATE link CONTENT $data";
    let data: BTreeMap<String, Value> =
        [("key".into(), key.into()), ("url".into(), url.into())].into();

    let vars: BTreeMap<String, Value> = [("data".into(), data.into())].into();

    let vec_res = ds.execute(sql, ses, Some(vars), false).await.unwrap();
    if let Ok(obj) = into_surreal_object(vec_res) {
        match obj.get("id") {
            Some(Value::Thing(id)) => Ok(id.to_raw()),
            _ => Err(SurrealError::NotFound),
        }
    } else {
        Err(SurrealError::InternalError)
    }
}

pub async fn find_link_by_key((ds, ses): &DB, key: &str) -> Result<String, SurrealError> {
    let sql = "SELECT * FROM link WHERE key = $key";
    let vars: BTreeMap<String, Value> = [("key".into(), key.into())].into();

    let vec_res = ds.execute(sql, ses, Some(vars), false).await.unwrap();
    let object_result = into_surreal_object(vec_res);

    if let Ok(object) = object_result {
        match object.get("url") {
            Some(Value::Strand(url)) => Ok(url.as_str().to_string()),
            _ => Err(SurrealError::NotFound),
        }
    } else {
        Err(SurrealError::InternalError)
    }
}

pub fn into_iter_objects(
    ress: Vec<SurrealResponse>,
) -> Result<impl Iterator<Item = Result<Object, SurrealError>>, SurrealError> {
    let res = ress
        .into_iter()
        .next()
        .map(|rp| rp.result)
        .transpose()
        .map_err(|_| SurrealError::OperationError)?;

    match res {
        Some(Value::Array(arr)) => {
            let it = arr.into_iter().map(|v| match v {
                Value::Object(object) => Ok(object),
                _ => Err(SurrealError::NotFound),
            });
            Ok(it)
        }
        _ => Err(SurrealError::InternalError),
    }
}

pub fn into_surreal_object(ress: Vec<SurrealResponse>) -> Result<Object, SurrealError> {
    let res = ress
        .into_iter()
        .next()
        .map(|rp| rp.result)
        .transpose()
        .map_err(|_| SurrealError::OperationError)?;

    if let Some(Value::Array(object_arr)) = res {
        match object_arr.into_iter().next() {
            Some(Value::Object(object)) => Ok(object),
            _ => Err(SurrealError::InternalError),
        }
    } else {
        Err(SurrealError::NotFound)
    }
}

#[derive(Debug)]
pub enum SurrealError {
    NotFound,
    InternalError,
    OperationError,
}
