use actix_web::HttpResponse;

pub async fn index() -> HttpResponse {
    HttpResponse::Ok().body(r#"<h1>Hello, CoffeeOJ!</h1>"#.to_string())
}
