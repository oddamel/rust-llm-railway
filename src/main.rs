use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

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

async fn health_check(data: web::Data<Arc<Instant>>) -> Result<HttpResponse> {
    let uptime = data.elapsed().as_secs();
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        service: "rust-llm-service".to_string(),
        version: "0.1.0".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        uptime_seconds: uptime,
    };
    
    info!("Health check requested - uptime: {}s", uptime);
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
    
    info!("Generated text response in {}ms", processing_time);
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
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🦀 Starting Rust LLM Service...");

    // Get configuration from environment
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3200".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid port number");

    info!("🚀 Rust LLM Service starting...");
    info!("   - Host: {}", host);
    info!("   - Port: {}", port);
    info!("   - Environment PORT: {:?}", env::var("PORT"));
    info!("   - Binding to: {}:{}", host, port);

    // Track startup time for uptime calculation
    let start_time = Arc::new(Instant::now());

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(start_time.clone()))
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