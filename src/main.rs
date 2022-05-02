mod db;

#[macro_use]
extern crate dotenv_codegen;

use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer};
use db::paste::Paste;
use dotenv::dotenv;
use futures::stream::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    Client, Collection,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DB_NAME: &str = "pastes";
const COLL_NAME: &str = "pastes";
const IP_PEPPER: &str = dotenv!("IP_PEPPER");

#[derive(Serialize, Deserialize)]
struct CreatePasteDto {
    pub title: String,
    pub content: String,
}

#[get("/author/{author_id}")]
async fn get_paste_by_author(
    coll: web::Data<Collection<Paste>>,
    author_id: web::Path<String>,
) -> HttpResponse {
    let post_cursor = coll
        .find(
            doc! {
                "author_id": author_id.to_string()
            },
            None,
        )
        .await
        .unwrap();

    let posts = post_cursor.try_collect::<Vec<Paste>>().await;

    match posts {
        Ok(posts) => {
            let mut body = format!("<h1>Found {} posts!</h1>", &posts.len());

            for i in &posts {
                body.push_str(&format!("<a href=\"/{id}\">{id}</a></br>", id = i.id));
            }

            HttpResponse::Ok().body(body)
        }
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

#[get("/{id}")]
async fn get_single_paste(
    coll: web::Data<Collection<Paste>>,
    id: web::Path<String>,
) -> HttpResponse {
    let bson_id = bson::oid::ObjectId::parse_str(id.to_string()).unwrap();
    let post = coll
        .find_one(
            doc! {
                "_id": bson_id,
            },
            None,
        )
        .await;

    match post {
        Ok(Some(post)) => HttpResponse::Ok().body(format!(
            "<h1>{}</h1><p>{}</p><code>{}</code>",
            post.title, post.content, post.author_id
        )),
        Ok(None) => HttpResponse::NotFound().body("<h1>Not Found</h1>"),
        Err(_) => HttpResponse::InternalServerError().body("<h1>Internal Server Error</h1>"),
    }
}

#[post("/")]
async fn create_paste(
    coll: web::Data<Collection<Paste>>,
    paste: web::Json<CreatePasteDto>,
    req: HttpRequest,
) -> HttpResponse {
    let ip = req.peer_addr();

    if ip.is_none() {
        return HttpResponse::InternalServerError().json("Internal server error");
    }

    let ip = ip.unwrap();

    let author_id = generate_user_id(&ip.to_string());

    let result = coll
        .insert_one(
            Paste {
                id: bson::oid::ObjectId::new(),
                title: paste.title.clone(),
                content: paste.content.clone(),
                author_id,
            },
            None,
        )
        .await;

    match result {
        Ok(result) => HttpResponse::Ok().json(result.inserted_id),
        Err(_e) => HttpResponse::InternalServerError().body("Failed to insert paste"),
    }
}

fn generate_user_id(ip: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{}{}", ip, IP_PEPPER)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let uri = dotenv!("MONGODB_URI");

    let client = Client::with_uri_str(uri).await.expect("failed to connect");

    let db = client.database(DB_NAME);

    let coll: Collection<Paste> = db.collection(COLL_NAME);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(coll.clone()))
            .service(create_paste)
            .service(get_single_paste)
            .service(get_paste_by_author)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
