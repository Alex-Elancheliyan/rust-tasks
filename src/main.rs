use axum::{extract::{State, ConnectInfo, Multipart},routing::{get, post},response::IntoResponse, Json, Router,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use serde::{Deserialize, Serialize};
use dotenv::dotenv;
use std::{env, net::SocketAddr};
use tokio::{fs, net::TcpListener};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Student {
    id: i32,
    full_name: String,
    email: String,
    course: String,
    age: i32,
    reg_time: DateTime<Utc>,
    ip_address: Option<String>,
    created_by: String,
    pdf_file: Option<String>,
}

// #[derive(Debug, Deserialize)] 
// struct NewStudent {
//     full_name: String,
//     email: String,
//     course: String,
//     age: i32,
//     created_by: i32,
// }

async fn get_students(State(pool): State<PgPool>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Student>("SELECT * FROM students").fetch_all(&pool).await;

    match result {
        Ok(students) => Json(students).into_response(),
        Err(err) => {
            println!("DB Fetch Error: {:?}", err);
            Json(Vec::<Student>::new()).into_response()
        }
    }
}

async fn add_student(
    State(pool): State<PgPool>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut full_name = String::new();
    let mut email = String::new();
    let mut course = String::new();
    let mut age: i32 = 0;
    let mut created_by_id: i32 = 0;
    let mut pdf_path: Option<String> = None;


    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();

        match name.as_str() {
            "full_name" => full_name = field.text().await.unwrap(),
            "email" => email = field.text().await.unwrap(),
            "course" => course = field.text().await.unwrap(),
            "age" => age = field.text().await.unwrap().parse().unwrap_or(0),
            "created_by" => created_by_id = field.text().await.unwrap().parse().unwrap_or(0),
            "pdf_file" => {

                let file_name = field.file_name().unwrap_or("upload.pdf").to_string();
                let file_bytes = field.bytes().await.unwrap();

                let save_dir = "uploads";
                fs::create_dir_all(save_dir).await.unwrap();
                let save_path = format!("{}/{}", save_dir, file_name);
                fs::write(&save_path, &file_bytes).await.unwrap();

                pdf_path = Some(save_path);
            }
            _ => {}
        }
    }

    let reg_time = Utc::now();
    let ip_address = Some(addr.ip().to_string());
    let created_by = match created_by_id {
        1 => "Admin".to_string(),
        2 => "SuperAdmin".to_string(),
        _ => "Unknown".to_string(),
    };

    let result = sqlx::query!(
        r#"
        INSERT INTO students (full_name, email, course, age, reg_time, ip_address, created_by, pdf_file)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        full_name,
        email,
        course,
        age,
        reg_time,
        ip_address,
        created_by,
        pdf_path,
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => (axum::http::StatusCode::OK, "Student added with file").into_response(),
        Err(err) => {
            println!("Insert Error: {:?}", err);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add student",
            )
                .into_response()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    let app = Router::new().route("/students", get(get_students).post(add_student)).with_state(pool)
        .into_make_service_with_connect_info::<SocketAddr>();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server running at http://{}", addr);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}