use anyhow::{anyhow, Result as AnyhowResult};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::{
    sql::{Object, Value},
    Datastore, Response as SurrealResponse, Session,
};

// TODO: Add one time setup, specifically setting unique constraint on key in link table
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let db: Arc<DB> = Arc::new((
        Datastore::new("file://links.db").await.unwrap(),
        Session::for_db("edge_go", "links"),
    ));

    let app = Router::new()
        .route("/", get(index))
        .route("/:redirect_url", get(redirect))
        .route("/new-link", post(new_link))
        .route("/links", get(list_links))
        .with_state(db.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 4545));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> impl IntoResponse {
    Json(HelloWorld {
        message: "Hello, World!".to_string(),
    })
}

async fn redirect(
    Path(redirect_url): Path<String>,
    State(db): State<Arc<DB>>,
) -> impl IntoResponse {
    if let Ok(link) = find_link_by_key(db.as_ref(), &redirect_url).await {
        let new_location = link.get("url").unwrap();
        tracing::debug!("Redirecting to {}", link);
    }

    // let location_header = HeaderValue::from_str(new_location).unwrap();

    // Response::builder()
    //     .status(StatusCode::SEE_OTHER)
    //     .header("Location", location_header)
    //     .body(Body::empty())
    //     .unwrap()
}

async fn new_link(State(db): State<Arc<DB>>, Json(new_link): Json<NewLink>) -> impl IntoResponse {
    let key = new_link.key;
    let url = new_link.url;

    match create_link(db.as_ref(), &key, &url).await {
        Ok(_) => Json(HelloWorld {
            message: "Link created!".to_string(),
        }),
        Err(_) => Json(HelloWorld {
            message: "Link not created!".to_string(),
        }),
    }
}

async fn list_links(State(db): State<Arc<DB>>) -> impl IntoResponse {
    let (ds, ses) = db.as_ref();
    let sql = "SELECT * FROM link";
    let ress = ds.execute(sql, ses, None, false).await.unwrap();

    let mut links: Vec<BTreeMap<String, Value>> = Vec::new();
    if let Ok(res) = into_iter_objects(ress) {
        res.for_each(|obj| {
            let mut link: BTreeMap<String, Value> = BTreeMap::new();
            for (k, v) in obj.unwrap() {
                link.insert(k, v);
            }
            links.push(link);
        });
    }
    Json(links)
}

async fn create_link((ds, ses): &DB, key: &str, url: &str) -> Result<(), ()> {
    let sql = "CREATE link CONTENT $data";
    let data: BTreeMap<String, Value> =
        [("key".into(), key.into()), ("url".into(), url.into())].into();

    let vars: BTreeMap<String, Value> = [("data".into(), data.into())].into();

    // TODO: Return id of created link
    let _ = ds.execute(sql, ses, Some(vars), false).await.unwrap();
    Ok(())
}

async fn find_link_by_key((ds, ses): &DB, key: &str) -> Result<Object, ()> {
    let sql = "SELECT * FROM link WHERE key = $key";
    let vars: BTreeMap<String, Value> = [("key".into(), key.into())].into();

    let ress = ds.execute(sql, ses, Some(vars), false).await.unwrap();
    let somethign = into_surreal_object(ress);
    tracing::debug!("Found link 1: {:?}", &somethign);
    if let Ok(res) = somethign {
        tracing::debug!("Found link: {:?}", res);
        Ok(res)
    } else {
        Err(())
    }
}

fn into_iter_objects(
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

fn into_surreal_object(ress: Vec<SurrealResponse>) -> AnyhowResult<Object> {
    let res_iter = into_iter_objects(&ress)?;
    let res = ress.into_iter().next().map(|rp| rp.result).transpose()?;
    tracing::debug!("Found link 2: {:?}", res_iter.count());
    match res {
        Some(Value::Object(object)) => Ok(object),
        _ => Err(anyhow!("No records found.")),
    }
}

#[derive(Serialize, Deserialize)]
struct HelloWorld {
    message: String,
}

#[derive(Deserialize)]
struct NewLink {
    key: String,
    url: String,
}

type DB = (Datastore, Session);
