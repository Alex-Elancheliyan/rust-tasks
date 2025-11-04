use axum::{ extract::{State, Json,ConnectInfo}, routing::{get}, Router, response::IntoResponse,};
use sqlx::{postgres::PgPoolOptions, PgPool};
use serde::{Deserialize, Serialize};
use dotenv::dotenv;
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;

use chrono::{DateTime, Utc};




#[derive(Debug,Serialize, Deserialize, sqlx::FromRow)]       // Similiar to PY DB Model/schema
struct Student {
    id: i32,
    full_name: String,
    email: String,
    course: String,
    age: i32,
    reg_time: DateTime<Utc>,
    ip_address: Option<String>,
    created_by: String,
}

#[derive(Debug, Deserialize)]
struct NewStudent {
    full_name: String,
    email: String,
    course: String,
    age: i32,
    created_by: i32,
}

async fn get_students(State(pool): State<PgPool>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Student>("SELECT * FROM students").fetch_all(&pool).await;

    match result { Ok(students) => Json(students),
        Err(err) => {
            println!(" DB Fetch Error: {:?}", err);
            Json(Vec::<Student>::new())
        }
    }
}


async fn add_student( State(pool): State<PgPool>,
                      ConnectInfo(addr): ConnectInfo<SocketAddr>, 
    Json(student): Json<NewStudent>,
) -> impl IntoResponse {
    let ip_address = Some(addr.ip().to_string()); 
     
    let reg_time = Utc::now();  
    let created_by = match student.created_by {
                               1 => "Admin".to_string(),
                               2 => "SuperAdmin".to_string(),
                               _ => "Unknown".to_string(),
    };

    let result = sqlx::query!(
        r#"
        INSERT INTO students (full_name, email, course, age, reg_time, ip_address, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        student.full_name , student.email, student.course,
        student.age, reg_time, ip_address, created_by
    ).execute(&pool).await;

    match result { Ok(_) => "Student added",
        Err(err) => {
            println!("Insert Error: {:?}", err);
            "Failed to add student."
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {

    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    let pool = PgPoolOptions::new().max_connections(5).connect(&db_url).await?;


    let app = Router::new().route("/students", get(get_students).post(add_student)).with_state(pool)
    .into_make_service_with_connect_info::<SocketAddr>();


    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

  
    let listener = TcpListener::bind(addr).await.unwrap();
    println!(" Server is http://{}", addr);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
