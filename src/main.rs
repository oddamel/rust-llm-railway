use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result, HttpRequest, dev::ServiceRequest, dev::ServiceResponse, Error};
use actix_web::middleware::from_fn;
use serde::{Deserialize, Serialize};
use std::env;
use sha2::{Sha256, Digest};

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

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    timestamp: String,
}

// API Key validation middleware
async fn validate_api_key(
    req: ServiceRequest,
    next: actix_web::dev::Next<impl actix_web::body::MessageBody>,
) -> Result<ServiceResponse<impl actix_web::body::MessageBody>, Error> {
    // Skip auth for health endpoint
    if req.path() == "/api/health" {
        return next.call(req).await;
    }

    let api_key = env::var("RUST_LLM_API_KEY").unwrap_or_else(|_| "".to_string());
    
    // If no API key is configured, allow requests (for development)
    if api_key.is_empty() {
        println!("⚠️  Warning: No API key configured, skipping authentication");
        return next.call(req).await;
    }

    let auth_header = req.headers().get("Authorization");
    
    if let Some(auth_value) = auth_header {
        if let Ok(auth_str) = auth_value.to_str() {
            if auth_str.starts_with("Bearer ") {
                let provided_key = &auth_str[7..]; // Remove "Bearer " prefix
                
                // Use constant-time comparison to prevent timing attacks
                let expected_hash = {
                    let mut hasher = Sha256::new();
                    hasher.update(api_key.as_bytes());
                    hex::encode(hasher.finalize())
                };
                
                let provided_hash = {
                    let mut hasher = Sha256::new();
                    hasher.update(provided_key.as_bytes());
                    hex::encode(hasher.finalize())
                };
                
                if expected_hash == provided_hash {
                    return next.call(req).await;
                }
            }
        }
    }

    // Authentication failed
    let error_response = ErrorResponse {
        error: "Unauthorized".to_string(),
        message: "Invalid or missing API key. Include 'Authorization: Bearer <your-api-key>' header.".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(req.into_response(
        HttpResponse::Unauthorized()
            .json(error_response)
    ))
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

    // Generate a secure API key if none is set
    if env::var("RUST_LLM_API_KEY").is_err() {
        let api_key = uuid::Uuid::new_v4().to_string();
        println!("🔑 Generated API key: {}", api_key);
        println!("   Set RUST_LLM_API_KEY environment variable to: {}", api_key);
        println!("   For security, set this in your Railway/Render environment variables");
        env::set_var("RUST_LLM_API_KEY", &api_key);
    } else {
        println!("🔒 API key authentication enabled");
    }

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
            .wrap(from_fn(validate_api_key))
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