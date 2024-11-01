mod svelte;

use axum::{
  http::{header::{self, HeaderMap}, StatusCode, Uri},
  response::{self, Html, IntoResponse, Response},
  routing::{get, post, Router},
  extract::State,
};
use rust_embed::RustEmbed;
use mime_guess;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use dotenvy::dotenv;
use std::env;
use anyhow::Result;
use tokio::time::Duration;
use tower_http::{
    services::ServeDir,
    compression::CompressionLayer,
    set_header::SetResponseHeaderLayer
};
use axum::http::{
  HeaderName,
  HeaderValue,
};
use std::str;
use tokio::fs::File;
use  tokio::io::AsyncWriteExt;

#[derive(Debug)]
struct CompasionResult {
  html: String,
  is_modified: bool
}

#[derive(Debug)]
struct Todos {
  description: String
}

#[derive(Debug, Clone)]
struct MyState {
  pool: SqlitePool
}

static INDEX_HTML: &str = "index.html";

#[derive(RustEmbed)]
#[folder = "src/svelte/build/"]
struct Assets;

#[tokio::main]
async fn main() -> Result<()> {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL")?;
  println!("Using database URL: {}", database_url);

  let pool = SqlitePoolOptions::new()
    .max_connections(10)
    .min_connections(5)
    .max_lifetime(Some(Duration::from_secs(60 * 60)))
    .idle_timeout(Some(Duration::from_secs(30)))
    .test_before_acquire(true)
    .acquire_timeout(Duration::from_secs(5))
    .connect(&database_url).await?;

  let my_state = MyState {
    pool: pool
  };

  // create_todos_table(&pool.clone()).await?;

  // let description = "hello";

  // let id = add_todo(&pool, description).await?;
  // println!("Added todo with id: {}", id);

  // list_todos(&pool).await?;

  let app = Router::new()
    .route("/api", get(post_users_handler))
    .nest_service("/assets", ServeDir::new("assets"))
    .fallback(static_handler)
    .layer(CompressionLayer::new().br(true))
    .layer(SetResponseHeaderLayer::overriding(
      HeaderName::from_static("cache-control"),
      HeaderValue::from_static("public, max-age=86400, immutable"),
  ))
  .with_state(my_state);

  let listener = tokio::net::TcpListener::bind("localhost:8000")
      .await
      .unwrap();
  println!("Listening on {}", listener.local_addr().unwrap());
  axum::serve(listener, app).await.unwrap();

  Ok(())
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
  let path = uri.path().trim_start_matches('/');

  if path.is_empty() || path == INDEX_HTML {
      return index_html().await;
  }

  match Assets::get(path) {
      Some(content) => {
          let mime = mime_guess::from_path(path).first_or_octet_stream();
          (
              [
                  (header::CONTENT_TYPE, mime.as_ref()),
                  // (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
              ],
              content.data,
          )
              .into_response()
      }
      None => {
        let path = format!("./frontend/index.html").to_owned();

        match tokio::fs::read_to_string(path.clone()).await {
          Ok(file) => {
              return Html(file).into_response();
          },
          Err(_) => {

            match Assets::get(&path) {
              Some(content) => {
                  if let Ok(html_str) = str::from_utf8(&content.data) {
                      Html(content.data).into_response()
                  } else {
                      Html(content.data).into_response()
                  }
              },
              None => not_found().await,
          }

          }
      }

    }
  }
}

async fn index_html() -> Response {
  match Assets::get(INDEX_HTML) {
      Some(content) => Html(content.data).into_response(),
      None => not_found().await,
  }
}

async fn not_found() -> Response {
  (StatusCode::NOT_FOUND, "404 Not Found").into_response()
}

async fn post_users_handler(
  State(state): State<MyState>,
) -> impl IntoResponse {
  let mut conn = match state.pool.acquire().await {
      Ok(conn) => conn,
      Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
  };

  match sqlx::query!(
      r#"
      update todos set description = 'tes 2' where id = 1
      "#
  )
  .execute(&mut *conn)
  .await 
  {
      Ok(hasil) => {
          println!("{:?}", hasil);
          let _ = list_todos(&state.pool).await.map_err(|e|  format!("Error: {}", e));
          compare_data("tes", "tes 2", "index.html").await;
          StatusCode::OK.into_response()
      }
      Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
  }
}

// async fn add_todo(pool: &SqlitePool, description: &str) -> Result<i64> {
//   let mut conn = pool.acquire().await.map_err(|e| format!("Error: {}", e));

//   let id = sqlx::query!(
//       r#"
//       INSERT INTO todos (description)
//       VALUES (?1)
//       "#,
//       description
//   )
//   .execute(&mut *conn)
//   .await?
//   .last_insert_rowid();

//   Ok(id)
// }

async fn list_todos(pool: &SqlitePool) -> Result<()> {
  let recs = sqlx::query!(
      r#"
      SELECT id, description, done
      FROM todos
      ORDER BY id
      "#
  )
  .fetch_all(pool)
  .await?;

  for rec in recs {
      println!(
          "- [{}] {}: {}",
          if rec.done { "x" } else { " " },
          rec.id,
          &rec.description,
      );
  }

  Ok(())
}

async fn create_todos_table(pool: &SqlitePool) -> Result<()> {
  let mut conn = pool.acquire().await?;
  
  sqlx::query!(
      r#"
      CREATE TABLE IF NOT EXISTS todos
(
    id          INTEGER PRIMARY KEY NOT NULL,
    description TEXT                NOT NULL,
    done        BOOLEAN             NOT NULL DEFAULT 0
)
      "#
  )
  .execute(&mut *conn)
  .await?;

  println!("Tabel 'todos' berhasil dibuat atau sudah ada.");

  Ok(())
}

async fn compare_data(data1: &str, data2: &str, path: &str) -> CompasionResult {
  if data1 == data2 {
    return CompasionResult {
      html: String::from("Default Html"),
      is_modified : false
    } 
  }

  let new_html = svelte::generate_svelte_template("navigate", data2, "index.html").await;
  // println!("{}", new_html);

  match File::create(format!("./frontend/{}", path)).await {
    Ok(mut file) => {
      match file.write_all(new_html.as_bytes()).await {
        Ok (_) => println!("File berhasil ditulis"),
        Err(e) => eprintln!("Error saat menulis file: {}", e)
      }
    },
    Err(e) => eprintln!("Error saat membuat file: {}", e)
  }

  return CompasionResult {
    html: new_html,
    is_modified: true
  }
}