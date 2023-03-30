use std::collections::BTreeMap;
use anyhow::{anyhow, Result as AnyhowResult};
use surrealdb::{
    sql::{Object, Value},
    Datastore, Response as SurrealResponse, Session,
};
pub type DB = (Datastore, Session);

pub async fn create_link((ds, ses): &DB, key: &str, url: &str) -> Result<String, ()> {
    let sql = "CREATE link CONTENT $data";
    let data: BTreeMap<String, Value> =
        [("key".into(), key.into()), ("url".into(), url.into())].into();

    let vars: BTreeMap<String, Value> = [("data".into(), data.into())].into();

    let vec_res = ds.execute(sql, ses, Some(vars), false).await.unwrap();
    if let Ok(obj) = into_surreal_object(vec_res) {
        match obj.get("id".into()) {
            Some(Value::Thing(id)) => Ok(id.to_raw()),
            _ => Err(()),
        }
    } else {
        Err(())
    }
}

pub fn into_iter_objects(
  ress: Vec<SurrealResponse>,
) -> AnyhowResult<impl Iterator<Item = AnyhowResult<Object>>> {
  let res = ress.into_iter().next().map(|rp| rp.result).transpose()?;

  match res {
      Some(Value::Array(arr)) => {
          let it = arr.into_iter().map(|v| match v {
              Value::Object(object) => Ok(object),
              _ => Err(anyhow!("A record was not an Object")),
          });
          Ok(it)
      }
      _ => Err(anyhow!("No records found.")),
  }
}

pub fn into_surreal_object(ress: Vec<SurrealResponse>) -> AnyhowResult<Object> {
  let res = ress.into_iter().next().map(|rp| rp.result).transpose()?;

  if let Some(Value::Array(object_arr)) = res {
      match object_arr.into_iter().next() {
          Some(Value::Object(object)) => Ok(object),
          _ => Err(anyhow!("A record was not an Object")),
      }
  } else {
      Err(anyhow!("No records found."))
  }
}