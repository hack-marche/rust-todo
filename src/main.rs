use actix_web::{
    web,
    get,
    post,
    App,
    Result,
    HttpRequest,
    HttpResponse,
    HttpServer,
    Responder,
    ResponseError,
    http::header
};
use thiserror::Error;
use askama::Template;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::Deserialize;

struct TodoEntry {
    id: u32,
    text: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    entries: Vec<TodoEntry>
}

#[derive(Error, Debug)]
enum MyError {
    #[error("Failed to render HTML")]
    AskamaError(#[from] askama::Error),

    #[error("Failed to get connection")]
    ConnectionPoolError(#[from] r2d2::Error),

    #[error("Failed SQL execution.")]
    SQLiteError(#[from] rusqlite::Error),
}

impl ResponseError for MyError {}

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

#[get("/todos")]
async fn todos(db: web::Data<Pool<SqliteConnectionManager>>) -> Result<HttpResponse, MyError> {
    let conn = db.get()?;
    let mut statement = conn.prepare("SELECT id, text FROM todo")?;
    let rows = statement.query_map(params![], |row| {
        let id = row.get(0)?;
        let text = row.get(1)?;
        println!("{}", id);
        Ok(TodoEntry { id, text })
    })?;
    let mut entries = Vec::new();
    // entries.push(TodoEntry {
    //     id: 1,
    //     text: "First Todo Entry.".to_string(),
    // });
    // entries.push(TodoEntry {
    //     id: 2,
    //     text: "Second Todo Entry.".to_string(),
    // });
    for row in rows {
        entries.push(row?);
    }
    let html = IndexTemplate { entries };
    let response_body = html.render()?;
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(response_body))
}

#[derive(Deserialize)]
struct AddParam {
    text: String
}

#[post("/add_todo")]
async fn add_todo(
    params: web::Form<AddParam>,
    db: web::Data<Pool<SqliteConnectionManager>>
) -> Result<HttpResponse, MyError> {
    let conn = db.get()?;
    conn.execute("INSERT INTO todo (text) VALUES (?)", &[&params.text])?;
    Ok(HttpResponse::SeeOther().header(header::LOCATION, "/todos").finish())
}

#[post("/delete_todo")]
async fn delete_todo(
    params: web::Form<DeleteParam>,
    db: web::Data<Pool<SqliteConnectionManager>>
) -> Result<HttpResponse, MyError> {
    let conn = db.get()?;
    conn.execute("DELETE FROM todo id = ?", &[&params.id])?;
    Ok(HttpResponse::SeeOther().header(header::LOCATION, "/todos").finish())
}

#[derive(Deserialize)]
struct DeleteParam {
    id: u32
}

#[get("/")]
async fn hello() -> Result<HttpResponse, MyError> {
    let response_body = "Hello world!";
    Ok(HttpResponse::Ok().body(response_body))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let manager = SqliteConnectionManager::file("todos.db");
    let pool = Pool::new(manager).expect("Failed to initialized connection pool.");
    let conn = pool.get().expect("Failed to get the connectioon from the pool.");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS todo (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL
        )",
        params![],
    ).expect("Failed to create table 'todo'.");
    
    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(todos)
            .service(add_todo)
            .service(delete_todo)
            .route("/{name}", web::get().to(greet))
            .data(pool.clone())
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}