use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use go_urls::surrealutils::{create_link, find_link_by_key, into_iter_objects, DB};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::{sql::Value, Datastore, Session};

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
        .with_state(db);

    let addr = SocketAddr::from(([127, 0, 0, 1], 4545));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> impl IntoResponse {
    Json(StringResponse {
        message: "Hello, World!".to_string(),
    })
}

async fn redirect(
    Path(redirect_url): Path<String>,
    State(db): State<Arc<DB>>,
) -> impl IntoResponse {
    if let Ok(link) = find_link_by_key(db.as_ref(), &redirect_url).await {
        let new_location = link.as_str();

        let location_header = HeaderValue::from_str(new_location).unwrap();

        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header("Location", location_header)
            .body(Body::empty())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()
    }
}

async fn new_link(State(db): State<Arc<DB>>, Json(new_link): Json<NewLink>) -> impl IntoResponse {
    let key = new_link.key;
    let url = new_link.url;

    match create_link(db.as_ref(), &key, &url).await {
        Ok(id) => Json(StringResponse { message: id }),
        Err(_) => Json(StringResponse {
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

#[derive(Serialize)]
struct StringResponse {
    message: String,
}

#[derive(Deserialize)]
struct NewLink {
    key: String,
    url: String,
}
