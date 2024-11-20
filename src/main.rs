use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, FromRow};
use std::env;
use actix_cors::Cors;
use dotenv::dotenv;

#[derive(Serialize, Deserialize, FromRow)]
struct Product {
    id: i32,
    name: String,
    price: f64,
    in_stock: bool,
}

#[derive(Serialize, Deserialize, FromRow)]
struct Sale {
    id: i32,
    product_id: i32,
    discount: i32,
    start_date: String,
    end_date: String,
}

 
async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL NOT NULL,
            in_stock BOOLEAN NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sales (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER,
            discount INTEGER,
            start_date TEXT,
            end_date TEXT,
            FOREIGN KEY(product_id) REFERENCES products(id)
        );
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_products(pool: web::Data<SqlitePool>) -> impl Responder {
    let products = sqlx::query_as::<_, Product>("SELECT * FROM products")
        .fetch_all(pool.get_ref())
        .await;

    match products {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn get_sales(pool: web::Data<SqlitePool>) -> impl Responder {
    let sales = sqlx::query_as::<_, Sale>("SELECT * FROM sales")
        .fetch_all(pool.get_ref())
        .await;

    match sales {
        Ok(sales) => HttpResponse::Ok().json(sales),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}


#[derive(Deserialize, Debug)]
struct AddProduct {
    name: String,
    price: f64,
    in_stock: bool,
}

async fn add_product(
    pool: web::Data<SqlitePool>,
    product: web::Json<AddProduct>,
) -> impl Responder {
    println!("Received product data: {:?}", product);

    let result = sqlx::query!(
        "INSERT INTO products (name, price, in_stock) VALUES (?, ?, ?)",
        product.name,
        product.price,
        product.in_stock
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().body("Product added successfully"),
        Err(e) => {
            println!("Error adding product: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Deserialize)]
struct AddSale {
    product_id: i32,
    discount: i32,
    start_date: String,
    end_date: String,
}

async fn add_sale(
    pool: web::Data<SqlitePool>,
    sale: web::Json<AddSale>,
) -> impl Responder {

    let product_exists = sqlx::query!("SELECT id FROM products WHERE id = ?", sale.product_id)
        .fetch_optional(pool.get_ref())
        .await;

    match product_exists {
        Ok(Some(_)) => {
            let result = sqlx::query!(
                "INSERT INTO sales (product_id, discount, start_date, end_date) VALUES (?, ?, ?, ?)",
                sale.product_id,
                sale.discount,
                sale.start_date,
                sale.end_date
            )
            .execute(pool.get_ref())
            .await;

            match result {
                Ok(_) => HttpResponse::Ok().body("Sale added successfully"),
                Err(e) => {
                    println!("Error adding sale: {}", e);
                    HttpResponse::InternalServerError().body("Error adding sale")
                }
            }
        }
        Ok(None) => HttpResponse::BadRequest().body("Product does not exist"),
        Err(e) => {
            println!("Error checking product: {}", e);
            HttpResponse::InternalServerError().body("Error checking product existence")
        }
    }
}


async fn delete_product(
    pool: web::Data<SqlitePool>,
    path: web::Path<i32>,
) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query!("DELETE FROM products WHERE id = ?", id)
        .execute(pool.get_ref())
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().body("Product deleted successfully"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn delete_sale(
    pool: web::Data<SqlitePool>,
    path: web::Path<i32>,
) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query!("DELETE FROM sales WHERE id = ?", id)
        .execute(pool.get_ref())
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().body("Sale deleted successfully"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = SqlitePool::connect(&database_url).await.expect("Failed to connect to DB");
    init_db(&pool).await.expect("Failed to initialize the database");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin() 
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors) 
            .app_data(web::Data::new(pool.clone()))
            .route("/products", web::get().to(get_products))
            .route("/add-product", web::post().to(add_product))
            .route("/delete-product/{id}", web::delete().to(delete_product))
            .route("/add-sale", web::post().to(add_sale))
            .route("/delete-sale/{id}", web::delete().to(delete_sale))
            .route("/sales", web::get().to(get_sales))
    })
    .bind("127.0.0.1:8082")?
    .run()
    .await
}
