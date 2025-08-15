use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize)]
struct TextGenerationRequest {
    prompt: String,
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct TextGenerationResponse {
    text: String,
    model: String,
    processing_time_ms: u64,
    tokens_generated: u32,
    timestamp: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    timestamp: String,
    uptime_seconds: u64,
}

async fn health_check() -> Result<HttpResponse> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        service: "rust-llm-service".to_string(),
        version: "0.1.0".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        uptime_seconds: 0,
    };
    
    println!("Health check requested");
    Ok(HttpResponse::Ok().json(response))
}

async fn text_generation(req: web::Json<TextGenerationRequest>) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();
    
    // Simulate AI text generation (replace with actual LLM in production)
    let generated_text = format!(
        "AI Response to '{}': This is a simulated response from the Rust LLM service. In a production environment, this would be replaced with actual LLM inference.",
        req.prompt
    );
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    let response = TextGenerationResponse {
        text: generated_text,
        model: req.model.clone().unwrap_or_else(|| "rust-llm-v1".to_string()),
        processing_time_ms: processing_time,
        tokens_generated: req.max_tokens.unwrap_or(100),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    println!("Generated text response in {}ms", processing_time);
    Ok(HttpResponse::Ok().json(response))
}

async fn list_models() -> Result<HttpResponse> {
    let models = serde_json::json!({
        "models": [
            {
                "id": "rust-llm-v1",
                "name": "Rust LLM v1.0",
                "description": "Production Rust-based language model",
                "max_tokens": 4096,
                "capabilities": ["text-generation", "completion"]
            }
        ],
        "total": 1,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(HttpResponse::Ok().json(models))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🦀 Starting Rust LLM Service...");

    // Get configuration from environment
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3200".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid port number");

    println!("🚀 Rust LLM Service starting...");
    println!("   - Host: {}", host);
    println!("   - Port: {}", port);
    println!("   - Environment PORT: {:?}", env::var("PORT"));
    println!("   - Binding to: {}:{}", host, port);

    // Start HTTP server
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .route("/api/health", web::get().to(health_check))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::scope("/inference")
                            .route("/text-generation", web::post().to(text_generation))
                    )
                    .service(
                        web::scope("/models")
                            .route("/list", web::get().to(list_models))
                    )
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}