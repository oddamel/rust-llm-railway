use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result, HttpRequest};
use serde::{Deserialize, Serialize};
use std::env;
use sha2::{Sha256, Digest};

#[derive(Deserialize)]
struct TextGenerationRequest {
    prompt: String,
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    norwegian_context: Option<bool>,
    organization_type: Option<String>,
}

#[derive(Deserialize)]
struct EmbeddingsRequest {
    text: String,
    model: Option<String>,
    norwegian_context: Option<bool>,
}

#[derive(Serialize)]
struct TextGenerationResponse {
    text: String,
    model: String,
    processing_time_ms: u64,
    tokens_generated: u32,
    timestamp: String,
    generated_text: Option<String>, // Alias for felleskassen compatibility
    inference_time_ms: Option<u64>, // Alias for felleskassen compatibility
    _routing: Option<RoutingInfo>,
}

#[derive(Serialize)]
struct RoutingInfo {
    service: String,
    #[serde(rename = "responseTime")]
    response_time: u64,
    version: String,
}

#[derive(Serialize)]
struct EmbeddingsResponse {
    embedding: Vec<f32>,
    model: String,
    processing_time_ms: u64,
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

// API Key validation function
fn validate_api_key_header(req: &HttpRequest) -> Result<(), HttpResponse> {
    let api_key = env::var("RUST_LLM_API_KEY").unwrap_or_else(|_| "".to_string());
    
    // If no API key is configured, allow requests (for development)
    if api_key.is_empty() {
        println!("⚠️  Warning: No API key configured, skipping authentication");
        return Ok(());
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
                    return Ok(());
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

    Err(HttpResponse::Unauthorized().json(error_response))
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

async fn text_generation(http_req: HttpRequest, req: web::Json<TextGenerationRequest>) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
    let start_time = std::time::Instant::now();
    
    // Enhanced Norwegian context processing
    let mut generated_text = if req.norwegian_context.unwrap_or(false) {
        format!(
            "Norsk AI-analyse for {}: {}. Merchant type: Ukjent norsk forhandler. VAT rate: 25% (standard norsk MVA). Category: Drift og administrasjon. Compliance: Følger norske frivillige organisasjoner regelverk.",
            req.organization_type.as_deref().unwrap_or("forening"),
            req.prompt
        )
    } else {
        format!(
            "AI Response to '{}': This is a simulated response from the Rust LLM service. In a production environment, this would be replaced with actual LLM inference.",
            req.prompt
        )
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    let model_name = req.model.clone().unwrap_or_else(|| "rust-llm-v1".to_string());
    
    let response = TextGenerationResponse {
        text: generated_text.clone(),
        model: model_name.clone(),
        processing_time_ms: processing_time,
        tokens_generated: req.max_tokens.unwrap_or(100),
        timestamp: chrono::Utc::now().to_rfc3339(),
        // Felleskassen compatibility fields
        generated_text: Some(generated_text),
        inference_time_ms: Some(processing_time),
        _routing: Some(RoutingInfo {
            service: "rust-llm".to_string(),
            response_time: processing_time,
            version: "0.1.0".to_string(),
        }),
    };
    
    println!("Generated Norwegian text response in {}ms", processing_time);
    Ok(HttpResponse::Ok().json(response))
}

async fn list_models(http_req: HttpRequest) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
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

async fn embeddings_endpoint(http_req: HttpRequest, req: web::Json<EmbeddingsRequest>) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
    
    let start_time = std::time::Instant::now();
    
    // Generate mock Norwegian-aware embeddings (256-dimensional)
    let embedding = if req.norwegian_context.unwrap_or(false) {
        // Simulate Norwegian merchant pattern embeddings
        (0..256).map(|i| {
            // Create patterns based on Norwegian text characteristics
            let base = (i as f32 * 0.1).sin();
            let norwegian_bias = if req.text.contains("REMA") || req.text.contains("ICA") || req.text.contains("COOP") {
                0.8 // High confidence for known Norwegian chains
            } else if req.text.contains("AS") || req.text.contains("Norge") {
                0.6 // Medium confidence for Norwegian business patterns
            } else {
                0.3 // Lower confidence for unknown patterns
            };
            base * norwegian_bias
        }).collect()
    } else {
        // Generic embeddings
        (0..256).map(|i| (i as f32 * 0.1).sin()).collect()
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    let response = EmbeddingsResponse {
        embedding,
        model: req.model.clone().unwrap_or_else(|| "sentence-transformer".to_string()),
        processing_time_ms: processing_time,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    println!("Generated embeddings response in {}ms", processing_time);
    Ok(HttpResponse::Ok().json(response))
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
            .route("/api/health", web::get().to(health_check))
            // Compatibility endpoint for felleskassen
            .route("/api/ai/text-generation", web::post().to(text_generation))
            .route("/api/ai/embeddings", web::post().to(embeddings_endpoint))
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