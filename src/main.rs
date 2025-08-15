use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result, HttpRequest};
use serde::{Deserialize, Serialize};
use std::env;
use std::collections::HashMap;
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

#[derive(Serialize, Clone)]
struct NorwegianMerchantInfo {
    name: String,
    chain: String,
    category: String,
    typical_vat_rate: u8,
    seasonal_products: Vec<String>,
    org_pattern: Option<String>,
    confidence: f32,
}

#[derive(Serialize)]
struct NorwegianAnalysis {
    merchant: NorwegianMerchantInfo,
    vat_analysis: VatAnalysis,
    seasonal_context: SeasonalContext,
    compliance_check: ComplianceCheck,
    cultural_significance: Option<String>,
    deductibility_assessment: String,
}

#[derive(Serialize)]
struct VatAnalysis {
    detected_rate: u8,
    rate_explanation: String,
    total_vat_amount: Option<f32>,
    compliance_status: String,
}

#[derive(Serialize)]
struct SeasonalContext {
    season: String,
    cultural_event: Option<String>,
    typical_purchases: Vec<String>,
    price_expectations: String,
}

#[derive(Serialize)]
struct ComplianceCheck {
    organization_type: String,
    deductibility: String,
    documentation_required: Vec<String>,
    approval_needed: bool,
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

// Norwegian Merchant Intelligence Database
fn get_norwegian_merchant_database() -> HashMap<&'static str, NorwegianMerchantInfo> {
    let mut merchants = HashMap::new();
    
    // REMA 1000 Intelligence
    merchants.insert("REMA", NorwegianMerchantInfo {
        name: "REMA 1000".to_string(),
        chain: "REMA 1000".to_string(),
        category: "Grocery Store".to_string(),
        typical_vat_rate: 15, // Food VAT
        seasonal_products: vec![
            "Ribbe".to_string(), "Pinnekjøtt".to_string(), "Lutefisk".to_string(),
            "Egg".to_string(), "Lam".to_string(), "Is".to_string(), "Grillmat".to_string()
        ],
        org_pattern: Some("999208372".to_string()),
        confidence: 0.95,
    });
    
    // ICA Intelligence
    merchants.insert("ICA", NorwegianMerchantInfo {
        name: "ICA Supermarket".to_string(),
        chain: "ICA".to_string(),
        category: "Grocery Store".to_string(),
        typical_vat_rate: 15,
        seasonal_products: vec![
            "Kvikk Lunsj".to_string(), "Egg".to_string(), "Melk".to_string(),
            "Brød".to_string(), "Ost".to_string()
        ],
        org_pattern: None,
        confidence: 0.92,
    });
    
    // COOP Intelligence  
    merchants.insert("COOP", NorwegianMerchantInfo {
        name: "Coop".to_string(),
        chain: "COOP".to_string(),
        category: "Grocery Store".to_string(),
        typical_vat_rate: 15,
        seasonal_products: vec![
            "Ø-merket".to_string(), "Miljømerket".to_string(), "Lokalt".to_string(),
            "Nærprodusert".to_string()
        ],
        org_pattern: None,
        confidence: 0.94,
    });
    
    // KIWI Intelligence
    merchants.insert("KIWI", NorwegianMerchantInfo {
        name: "KIWI".to_string(),
        chain: "KIWI".to_string(),
        category: "Discount Grocery".to_string(),
        typical_vat_rate: 15,
        seasonal_products: vec![
            "Lavpris".to_string(), "Tilbud".to_string(), "2 for 1".to_string()
        ],
        org_pattern: None,
        confidence: 0.93,
    });
    
    // Norwegian Gas Stations
    merchants.insert("CIRCLE K", NorwegianMerchantInfo {
        name: "Circle K".to_string(),
        chain: "Circle K".to_string(),
        category: "Gas Station".to_string(),
        typical_vat_rate: 25, // General VAT
        seasonal_products: vec![
            "Bensin".to_string(), "Diesel".to_string(), "Kaffe".to_string(),
            "Pølse".to_string(), "Brus".to_string()
        ],
        org_pattern: None,
        confidence: 0.88,
    });
    
    merchants.insert("SHELL", NorwegianMerchantInfo {
        name: "Shell".to_string(),
        chain: "Shell".to_string(),
        category: "Gas Station".to_string(),
        typical_vat_rate: 25,
        seasonal_products: vec![
            "Drivstoff".to_string(), "Bil".to_string(), "Kaffe".to_string()
        ],
        org_pattern: None,
        confidence: 0.87,
    });
    
    // Norwegian Brands and Stores
    merchants.insert("TINE", NorwegianMerchantInfo {
        name: "Tine".to_string(),
        chain: "Tine".to_string(),
        category: "Dairy Products".to_string(),
        typical_vat_rate: 15,
        seasonal_products: vec![
            "Melk".to_string(), "Yoghurt".to_string(), "Ost".to_string(),
            "Smør".to_string(), "Fløte".to_string()
        ],
        org_pattern: None,
        confidence: 0.98,
    });
    
    merchants.insert("POSTEN", NorwegianMerchantInfo {
        name: "Posten Norge".to_string(),
        chain: "Posten".to_string(),
        category: "Postal Service".to_string(),
        typical_vat_rate: 25,
        seasonal_products: vec![
            "Porto".to_string(), "Pakke".to_string(), "Brev".to_string()
        ],
        org_pattern: Some("984661185".to_string()),
        confidence: 0.99,
    });
    
    merchants.insert("VINMONOPOLET", NorwegianMerchantInfo {
        name: "Vinmonopolet".to_string(),
        chain: "Vinmonopolet".to_string(),
        category: "Alcohol Monopoly".to_string(),
        typical_vat_rate: 25, // Plus special alcohol taxes
        seasonal_products: vec![
            "Vin".to_string(), "Øl".to_string(), "Brennevin".to_string(),
            "Champagne".to_string(), "Akevitt".to_string()
        ],
        org_pattern: Some("971425831".to_string()),
        confidence: 0.99,
    });
    
    merchants
}

// Norwegian Business Pattern Recognition
fn detect_norwegian_merchant(text: &str) -> Option<NorwegianMerchantInfo> {
    let merchants = get_norwegian_merchant_database();
    let text_upper = text.to_uppercase();
    
    // Check for exact chain matches
    for (key, merchant) in &merchants {
        if text_upper.contains(key) {
            return Some(merchant.clone());
        }
    }
    
    // Check for specific Norwegian patterns
    if text_upper.contains("REMA 1000") || text_upper.contains("REMA1000") {
        return merchants.get("REMA").cloned();
    }
    
    if text_upper.contains("ICA SUPERMARKET") || text_upper.contains("ICA MAXI") {
        return merchants.get("ICA").cloned();
    }
    
    if text_upper.contains("COOP EXTRA") || text_upper.contains("COOP MEGA") || text_upper.contains("COOP PRIX") {
        return merchants.get("COOP").cloned();
    }
    
    if text_upper.contains("POSTEN NORGE") || text_upper.contains("POST NORGE") {
        return merchants.get("POSTEN").cloned();
    }
    
    // Organization number patterns
    for (_, merchant) in &merchants {
        if let Some(org_pattern) = &merchant.org_pattern {
            if text.contains(org_pattern) {
                return Some(merchant.clone());
            }
        }
    }
    
    None
}

// Norwegian Seasonal Analysis
fn get_seasonal_context(date_str: Option<&str>) -> SeasonalContext {
    use chrono::{NaiveDate, Datelike};
    
    let now = chrono::Utc::now();
    let month = now.month();
    let day = now.day();
    
    match month {
        5 if day == 17 => SeasonalContext {
            season: "17. mai (Constitution Day)".to_string(),
            cultural_event: Some("Norwegian National Day".to_string()),
            typical_purchases: vec![
                "Flagg".to_string(), "Korv".to_string(), "Brus".to_string(),
                "Is".to_string(), "Bunad tilbehør".to_string()
            ],
            price_expectations: "Premium pricing for patriotic items".to_string(),
        },
        12 => SeasonalContext {
            season: "Jul (Christmas)".to_string(),
            cultural_event: Some("Norwegian Christmas".to_string()),
            typical_purchases: vec![
                "Ribbe".to_string(), "Pinnekjøtt".to_string(), "Lutefisk".to_string(),
                "Lefse".to_string(), "Krumkake".to_string(), "Julepresanger".to_string()
            ],
            price_expectations: "High seasonal pricing for traditional foods".to_string(),
        },
        3..=4 => SeasonalContext {
            season: "Påske (Easter)".to_string(),
            cultural_event: Some("Norwegian Easter".to_string()),
            typical_purchases: vec![
                "Egg".to_string(), "Lam".to_string(), "Kvikk Lunsj".to_string(),
                "Påskeegg".to_string(), "Sjokolade".to_string()
            ],
            price_expectations: "Elevated prices for Easter chocolate and lamb".to_string(),
        },
        6..=8 => SeasonalContext {
            season: "Sommer (Summer)".to_string(),
            cultural_event: Some("Norwegian Summer Vacation".to_string()),
            typical_purchases: vec![
                "Is".to_string(), "Grillmat".to_string(), "Brus".to_string(),
                "Øl".to_string(), "Solkrem".to_string(), "Camping utstyr".to_string()
            ],
            price_expectations: "Peak pricing for summer and vacation items".to_string(),
        },
        9 => SeasonalContext {
            season: "Skolestart (Back to School)".to_string(),
            cultural_event: Some("Norwegian School Year Start".to_string()),
            typical_purchases: vec![
                "Skolesekk".to_string(), "Blyanter".to_string(), "Bøker".to_string(),
                "Matboks".to_string(), "Klær".to_string()
            ],
            price_expectations: "Back-to-school promotions and bulk pricing".to_string(),
        },
        _ => SeasonalContext {
            season: "Standard periode".to_string(),
            cultural_event: None,
            typical_purchases: vec![
                "Dagligvarer".to_string(), "Mat".to_string(), "Drikke".to_string()
            ],
            price_expectations: "Regular pricing".to_string(),
        },
    }
}

// Norwegian VAT Analysis
fn analyze_norwegian_vat(amount: f32, merchant: &NorwegianMerchantInfo, items: &str) -> VatAnalysis {
    let detected_rate = if items.to_lowercase().contains("melk") || 
                         items.to_lowercase().contains("brød") ||
                         items.to_lowercase().contains("mat") ||
                         merchant.category == "Grocery Store" {
        15 // Food VAT rate
    } else if merchant.chain == "Vinmonopolet" {
        25 // Alcohol gets 25% + special taxes
    } else {
        25 // General VAT rate
    };
    
    let vat_amount = amount * (detected_rate as f32 / (100.0 + detected_rate as f32));
    
    let rate_explanation = match detected_rate {
        0 => "VAT-exempt goods (books, newspapers, medicine)".to_string(),
        15 => "Reduced VAT rate for food and non-alcoholic beverages".to_string(),
        25 => "Standard VAT rate for general goods and services".to_string(),
        _ => "Special VAT rate".to_string(),
    };
    
    let compliance_status = if detected_rate == merchant.typical_vat_rate {
        "Compliant with expected rate".to_string()
    } else {
        format!("Rate differs from typical {}% for {}", merchant.typical_vat_rate, merchant.chain)
    };
    
    VatAnalysis {
        detected_rate,
        rate_explanation,
        total_vat_amount: Some(vat_amount),
        compliance_status,
    }
}

// Extract amount from Norwegian text
fn extract_amount_from_text(text: &str) -> Option<f32> {
    use regex::Regex;
    
    // Common Norwegian amount patterns
    let patterns = vec![
        r"(\d+[,.]?\d*)\s*(?:NOK|kr|kroner)", // 245.50 NOK, 156,90 kr
        r"(\d+[,.]?\d*)\s*(?:,-|:-)?\s*$",    // 245.50 at end of line
        r"TOTALT?\s*[:|]?\s*(\d+[,.]?\d*)",   // TOTALT: 245.50
        r"SUM\w*\s*[:|]?\s*(\d+[,.]?\d*)",    // SUMMA: 245.50
    ];
    
    for pattern_str in &patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            if let Some(caps) = re.captures(text) {
                if let Some(amount_str) = caps.get(1) {
                    let amount_text = amount_str.as_str().replace(',', ".");
                    if let Ok(amount) = amount_text.parse::<f32>() {
                        return Some(amount);
                    }
                }
            }
        }
    }
    
    None
}

// Norwegian Organization Compliance Check
fn check_norwegian_compliance(org_type: &str, merchant: &NorwegianMerchantInfo, amount: f32) -> ComplianceCheck {
    let mut documentation_required = vec!["Kvittering".to_string()];
    let mut approval_needed = false;
    
    let deductibility = match org_type {
        "forening" | "lag" | "klubb" => {
            if merchant.category == "Grocery Store" {
                documentation_required.push("Formål dokumentasjon".to_string());
                "Delvis fradragsberettiget - kun aktivitetsrelaterte innkjøp"
            } else if amount > 5000.0 {
                approval_needed = true;
                documentation_required.push("Styregodkjenning".to_string());
                "Krever styregodkjenning for beløp over 5000 NOK"
            } else {
                "Fradragsberettiget for organisasjonsaktivitet"
            }
        },
        "korps" => {
            if merchant.category == "Alcohol Monopoly" {
                "Ikke fradragsberettiget - alkohol ikke tillatt for korps"
            } else {
                documentation_required.push("Aktivitetsbevis".to_string());
                "Fradragsberettiget for korpsaktivitet"
            }
        },
        _ => "Kontakt regnskapsfører for vurdering"
    };
    
    if amount > 1000.0 {
        documentation_required.push("Bilagsnummer".to_string());
        documentation_required.push("Dato og formål".to_string());
    }
    
    ComplianceCheck {
        organization_type: org_type.to_string(),
        deductibility: deductibility.to_string(),
        documentation_required,
        approval_needed,
    }
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
    
    // Enhanced Norwegian context processing with comprehensive intelligence
    let generated_text = if req.norwegian_context.unwrap_or(false) {
        // Norwegian Business Intelligence Analysis
        let org_type = req.organization_type.as_deref().unwrap_or("forening");
        
        // Try to extract amount from prompt
        let amount = extract_amount_from_text(&req.prompt).unwrap_or(100.0);
        
        // Detect Norwegian merchant
        let merchant = detect_norwegian_merchant(&req.prompt).unwrap_or_else(|| {
            NorwegianMerchantInfo {
                name: "Ukjent norsk forhandler".to_string(),
                chain: "Generisk".to_string(),
                category: "Uidentifisert".to_string(),
                typical_vat_rate: 25,
                seasonal_products: vec![],
                org_pattern: None,
                confidence: 0.5,
            }
        });
        
        // Get seasonal context
        let seasonal = get_seasonal_context(None);
        
        // Analyze VAT
        let vat_analysis = analyze_norwegian_vat(amount, &merchant, &req.prompt);
        
        // Check compliance
        let compliance = check_norwegian_compliance(org_type, &merchant, amount);
        
        // Determine cultural significance
        let cultural_significance = if seasonal.cultural_event.is_some() {
            Some(format!("Kulturell betydning: {} - typiske innkjøp inkluderer {}",
                seasonal.cultural_event.as_ref().unwrap(),
                seasonal.typical_purchases.join(", ")
            ))
        } else {
            None
        };
        
        // Generate comprehensive Norwegian analysis
        let analysis = NorwegianAnalysis {
            merchant: merchant.clone(),
            vat_analysis,
            seasonal_context: seasonal,
            compliance_check: compliance,
            cultural_significance,
            deductibility_assessment: if merchant.category == "Alcohol Monopoly" && org_type == "korps" {
                "IKKE FRADRAGSBERETTIGET - Alkohol ikke tillatt for korps".to_string()
            } else if amount > 5000.0 {
                "Krever styregodkjenning for beløp over 5000 NOK".to_string()
            } else {
                "Fradragsberettiget for organisasjonsformål".to_string()
            },
        };
        
        // Format the comprehensive analysis
        format!(
            "🇳🇴 NORSK AI-ANALYSE FOR {} 🇳🇴\n\nMERCHANT: {} ({})\n├─ Kategori: {}\n├─ Konfidensgrad: {:.1}%\n├─ Forventet MVA: {}%\n\nMVA-ANALYSE:\n├─ Detektert sats: {}%\n├─ Forklaring: {}\n├─ MVA-beløp: {:.2} NOK\n├─ Status: {}\n\nSESONGANALYSE:\n├─ Periode: {}\n├─ Kulturell kontekst: {}\n├─ Typiske innkjøp: {}\n├─ Prisforventning: {}\n\nKOMPLIANCE FOR {}:\n├─ Fradragsberettighet: {}\n├─ Dokumentasjon påkrevd: {}\n├─ Styregodkjenning: {}\n\n{}ORIGINAL PROMPT: {}",
            org_type.to_uppercase(),
            analysis.merchant.name,
            analysis.merchant.chain,
            analysis.merchant.category,
            analysis.merchant.confidence * 100.0,
            analysis.merchant.typical_vat_rate,
            analysis.vat_analysis.detected_rate,
            analysis.vat_analysis.rate_explanation,
            analysis.vat_analysis.total_vat_amount.unwrap_or(0.0),
            analysis.vat_analysis.compliance_status,
            analysis.seasonal_context.season,
            analysis.seasonal_context.cultural_event.as_deref().unwrap_or("Ingen spesiell"),
            analysis.seasonal_context.typical_purchases.join(", "),
            analysis.seasonal_context.price_expectations,
            org_type.to_uppercase(),
            analysis.compliance_check.deductibility,
            analysis.compliance_check.documentation_required.join(", "),
            if analysis.compliance_check.approval_needed { "JA" } else { "NEI" },
            if let Some(cultural) = analysis.cultural_significance {
                format!("{}\n\n", cultural)
            } else {
                String::new()
            },
            req.prompt
        )
    } else {
        format!(
            "AI Response to '{}': This is a simulated response from the Rust LLM service. In a production environment, this would be replaced with actual LLM inference.",
            req.prompt
        )
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    let model_name = req.model.clone().unwrap_or_else(|| "rust-llm-norwegian-v1".to_string());
    
    let response = TextGenerationResponse {
        text: generated_text.clone(),
        model: model_name.clone(),
        processing_time_ms: processing_time,
        tokens_generated: req.max_tokens.unwrap_or(100),
        timestamp: chrono::Utc::now().to_rfc3339(),
        // Felleskassen compatibility fields
        generated_text: Some(generated_text.clone()),
        inference_time_ms: Some(processing_time),
        _routing: Some(RoutingInfo {
            service: "rust-llm-norwegian-intelligence".to_string(),
            response_time: processing_time,
            version: "2.0.0".to_string(),
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

    // Load .env file if it exists (for local development)
    dotenv::dotenv().ok();

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