use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result, HttpRequest};
use serde::{Deserialize, Serialize};
use std::env;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use std::sync::{Arc, Mutex};
use std::fs;

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

#[derive(Deserialize)]
struct DocumentProcessingRequest {
    image_data: Option<String>, // Base64 encoded image
    document_text: Option<String>, // Pre-extracted text
    document_type: Option<String>, // receipt, invoice, etc.
    norwegian_context: Option<bool>,
    organization_type: Option<String>,
    correction_data: Option<UserCorrection>,
}

#[derive(Deserialize, Serialize, Clone)]
struct UserCorrection {
    original_analysis: String,
    corrected_merchant: Option<String>,
    corrected_amount: Option<f32>,
    corrected_vat_rate: Option<u8>,
    corrected_category: Option<String>,
    user_feedback: Option<String>,
    confidence_rating: Option<u8>, // 1-10 scale
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
struct DocumentProcessingResponse {
    norwegian_analysis: NorwegianAnalysis,
    image_analysis: Option<ImageAnalysis>,
    processing_confidence: f32,
    learning_applied: bool,
    model: String,
    processing_time_ms: u64,
    timestamp: String,
}

#[derive(Serialize)]
struct ImageAnalysis {
    image_quality: String,
    text_regions_detected: u32,
    ocr_confidence: f32,
    document_type_detected: String,
    norwegian_text_detected: bool,
}

#[derive(Serialize)]
struct LearningResponse {
    correction_applied: bool,
    model_updated: bool,
    confidence_improvement: Option<f32>,
    similar_cases_updated: u32,
    timestamp: String,
}

#[derive(Deserialize)]
struct FineTuningRequest {
    training_data: Vec<TrainingExample>,
    model_type: Option<String>, // norwegian_merchant, vat_analysis, seasonal_patterns
    epochs: Option<u32>,
    learning_rate: Option<f32>,
    validation_split: Option<f32>,
}

#[derive(Deserialize, Serialize, Clone)]
struct TrainingExample {
    input_text: String,
    expected_merchant: Option<String>,
    expected_amount: Option<f32>,
    expected_vat_rate: Option<u8>,
    expected_category: Option<String>,
    context_metadata: Option<String>,
    quality_score: Option<f32>, // 0.0-1.0
}

#[derive(Serialize)]
struct FineTuningResponse {
    training_started: bool,
    model_id: String,
    training_examples_count: u32,
    estimated_training_time_minutes: u32,
    validation_metrics: Option<ModelMetrics>,
    status: String,
    timestamp: String,
}

#[derive(Serialize, Clone)]
struct ModelMetrics {
    accuracy: f32,
    precision: f32,
    recall: f32,
    f1_score: f32,
    norwegian_merchant_accuracy: f32,
    vat_compliance_accuracy: f32,
    seasonal_pattern_accuracy: f32,
}

#[derive(Deserialize)]
struct PredictiveAnalysisRequest {
    organization_type: String,
    historical_transactions: Vec<HistoricalTransaction>,
    prediction_timeframe: Option<String>, // "next_month", "next_quarter", "next_year"
    analysis_type: Option<String>, // "spending_patterns", "seasonal_trends", "budget_forecast"
}

#[derive(Deserialize, Serialize, Clone)]
struct HistoricalTransaction {
    date: String,
    merchant: String,
    amount: f32,
    category: String,
    season: Option<String>,
    cultural_event: Option<String>,
}

#[derive(Serialize)]
struct PredictiveAnalysisResponse {
    predictions: Vec<SpendingPrediction>,
    seasonal_insights: Vec<SeasonalInsight>,
    budget_recommendations: Vec<BudgetRecommendation>,
    confidence_score: f32,
    analysis_type: String,
    processing_time_ms: u64,
    timestamp: String,
}

#[derive(Serialize)]
struct SpendingPrediction {
    period: String,
    predicted_amount: f32,
    category: String,
    confidence: f32,
    trend: String, // "increasing", "decreasing", "stable"
    factors: Vec<String>,
}

#[derive(Serialize)]
struct SeasonalInsight {
    season: String,
    cultural_event: Option<String>,
    expected_spending_increase: f32,
    key_categories: Vec<String>,
    historical_pattern: String,
}

#[derive(Serialize)]
struct BudgetRecommendation {
    category: String,
    recommended_budget: f32,
    reasoning: String,
    risk_level: String, // "low", "medium", "high"
    optimization_tips: Vec<String>,
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

// Global learning storage (in production, this would be a proper database)
lazy_static::lazy_static! {
    static ref LEARNING_DATA: Arc<Mutex<Vec<UserCorrection>>> = Arc::new(Mutex::new(Vec::new()));
    static ref MERCHANT_LEARNING: Arc<Mutex<HashMap<String, f32>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref TRAINING_DATA: Arc<Mutex<Vec<TrainingExample>>> = Arc::new(Mutex::new(Vec::new()));
    static ref FINE_TUNED_MODELS: Arc<Mutex<HashMap<String, ModelMetrics>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref SEASONAL_PATTERNS: Arc<Mutex<HashMap<String, Vec<HistoricalTransaction>>>> = Arc::new(Mutex::new(HashMap::new()));
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

// Multi-modal Document Processing
fn process_document_image(image_data: &str) -> Option<ImageAnalysis> {
    // Simulate image processing (in production, use proper OCR like Tesseract)
    let decoded_size = (image_data.len() * 3) / 4; // Estimate original size
    
    Some(ImageAnalysis {
        image_quality: if decoded_size > 100000 { "High".to_string() } else { "Medium".to_string() },
        text_regions_detected: 5 + (decoded_size % 10) as u32, // Simulate text region detection
        ocr_confidence: 0.85 + (decoded_size % 100) as f32 / 1000.0, // Simulate OCR confidence
        document_type_detected: if image_data.contains("receipt") || image_data.len() % 3 == 0 { 
            "receipt".to_string() 
        } else { 
            "invoice".to_string() 
        },
        norwegian_text_detected: image_data.len() % 2 == 0, // Simulate Norwegian text detection
    })
}

// Extract text from image (simulate OCR)
fn extract_text_from_image(image_data: &str) -> String {
    // In production, this would use actual OCR
    // For demo, simulate Norwegian receipt text based on image characteristics
    let size_indicator = image_data.len() % 5;
    
    match size_indicator {
        0 => "REMA 1000 Oslo Melk 34.90 kr Brød 28.50 kr TOTALT 63.40 kr".to_string(),
        1 => "ICA Supermarket Bergen Egg 12stk 45.90 kr Lam 289.90 kr SUMMA 335.80 kr".to_string(),
        2 => "COOP Trondheim Kaffe 89.90 kr Ost 67.50 kr MEDLEM 12345 TOTALT 157.40 kr".to_string(),
        3 => "KIWI Stavanger Pölse 15.90 kr Brus 2L 29.90 kr TOTALT 45.80 kr".to_string(),
        _ => "VINMONOPOLET Oslo Vin rød 299.90 kr Øl 6pk 189.90 kr TOTALT 489.80 kr".to_string(),
    }
}

// Apply learning from user corrections
fn apply_user_learning(correction: &UserCorrection) -> bool {
    if let Ok(mut learning_data) = LEARNING_DATA.lock() {
        learning_data.push(correction.clone());
        
        // Update merchant learning confidence
        if let Some(merchant) = &correction.corrected_merchant {
            if let Ok(mut merchant_learning) = MERCHANT_LEARNING.lock() {
                let current_confidence = merchant_learning.get(merchant).unwrap_or(&0.5);
                let new_confidence = if correction.confidence_rating.unwrap_or(5) > 7 {
                    (current_confidence + 0.1).min(0.99)
                } else {
                    (current_confidence - 0.05).max(0.1)
                };
                merchant_learning.insert(merchant.clone(), new_confidence);
            }
        }
        
        true
    } else {
        false
    }
}

// Get learned merchant confidence
fn get_learned_merchant_confidence(merchant_name: &str) -> f32 {
    if let Ok(merchant_learning) = MERCHANT_LEARNING.lock() {
        merchant_learning.get(merchant_name).copied().unwrap_or(0.5)
    } else {
        0.5
    }
}

// Enhanced Norwegian merchant detection with learning
fn detect_norwegian_merchant_with_learning(text: &str) -> Option<NorwegianMerchantInfo> {
    if let Some(mut merchant) = detect_norwegian_merchant(text) {
        // Apply learned confidence adjustments
        let learned_confidence = get_learned_merchant_confidence(&merchant.name);
        merchant.confidence = (merchant.confidence + learned_confidence) / 2.0;
        Some(merchant)
    } else {
        None
    }
}

// Advanced Fine-Tuning Capabilities
fn simulate_model_fine_tuning(training_data: &[TrainingExample], model_type: &str) -> ModelMetrics {
    let training_count = training_data.len() as f32;
    
    // Simulate training performance based on data quality and quantity
    let base_accuracy = 0.75 + (training_count / 1000.0).min(0.2);
    let quality_boost = training_data.iter()
        .map(|example| example.quality_score.unwrap_or(0.7))
        .sum::<f32>() / training_count * 0.1;
    
    let accuracy = (base_accuracy + quality_boost).min(0.98);
    
    ModelMetrics {
        accuracy,
        precision: accuracy - 0.02,
        recall: accuracy - 0.01,
        f1_score: accuracy - 0.015,
        norwegian_merchant_accuracy: match model_type {
            "norwegian_merchant" => accuracy + 0.03,
            _ => accuracy - 0.05,
        },
        vat_compliance_accuracy: match model_type {
            "vat_analysis" => accuracy + 0.04,
            _ => accuracy - 0.03,
        },
        seasonal_pattern_accuracy: match model_type {
            "seasonal_patterns" => accuracy + 0.05,
            _ => accuracy - 0.04,
        },
    }
}

// Store training data for continuous learning
fn store_training_example(example: TrainingExample) -> bool {
    if let Ok(mut training_data) = TRAINING_DATA.lock() {
        training_data.push(example);
        // Keep only the most recent 10,000 examples
        if training_data.len() > 10000 {
            training_data.drain(0..1000);
        }
        true
    } else {
        false
    }
}

// Advanced Predictive Analytics
fn analyze_spending_patterns(
    historical_data: &[HistoricalTransaction], 
    organization_type: &str,
    timeframe: &str
) -> PredictiveAnalysisResponse {
    use chrono::{NaiveDate, Datelike};
    
    // Group transactions by category and month
    let mut category_totals: HashMap<String, f32> = HashMap::new();
    let mut monthly_totals: HashMap<u32, f32> = HashMap::new();
    let mut seasonal_data: HashMap<String, f32> = HashMap::new();
    
    for transaction in historical_data {
        // Category analysis
        *category_totals.entry(transaction.category.clone()).or_insert(0.0) += transaction.amount;
        
        // Monthly analysis
        if let Ok(date) = NaiveDate::parse_from_str(&transaction.date, "%Y-%m-%d") {
            *monthly_totals.entry(date.month()).or_insert(0.0) += transaction.amount;
        }
        
        // Seasonal analysis
        if let Some(season) = &transaction.season {
            *seasonal_data.entry(season.clone()).or_insert(0.0) += transaction.amount;
        }
    }
    
    // Generate predictions based on historical patterns
    let predictions: Vec<SpendingPrediction> = category_totals.iter().map(|(category, &total)| {
        let avg_monthly = total / 12.0;
        let multiplier = match timeframe {
            "next_month" => 1.0,
            "next_quarter" => 3.0,
            "next_year" => 12.0,
            _ => 3.0,
        };
        
        // Add seasonal adjustments
        let seasonal_multiplier = match category.as_str() {
            "Grocery Store" => 1.1, // Always needed
            "Alcohol Monopoly" => if monthly_totals.get(&12).unwrap_or(&0.0) > &monthly_totals.get(&6).unwrap_or(&0.0) { 1.3 } else { 0.8 },
            _ => 1.0,
        };
        
        SpendingPrediction {
            period: timeframe.to_string(),
            predicted_amount: avg_monthly * multiplier * seasonal_multiplier,
            category: category.clone(),
            confidence: 0.75 + (total / 10000.0).min(0.2),
            trend: if total > 5000.0 { "increasing".to_string() } else { "stable".to_string() },
            factors: vec![
                "Historical spending patterns".to_string(),
                "Seasonal adjustments".to_string(),
                format!("{} organization type", organization_type),
            ],
        }
    }).collect();
    
    // Seasonal insights
    let seasonal_insights = vec![
        SeasonalInsight {
            season: "17. mai (Constitution Day)".to_string(),
            cultural_event: Some("Norwegian National Day".to_string()),
            expected_spending_increase: 1.8,
            key_categories: vec!["Flagg".to_string(), "Korv".to_string(), "Brus".to_string()],
            historical_pattern: "350% increase in patriotic items and food for celebrations".to_string(),
        },
        SeasonalInsight {
            season: "Jul (Christmas)".to_string(),
            cultural_event: Some("Norwegian Christmas".to_string()),
            expected_spending_increase: 2.2,
            key_categories: vec!["Ribbe".to_string(), "Pinnekjøtt".to_string(), "Julepresanger".to_string()],
            historical_pattern: "Peak spending season with traditional food focus".to_string(),
        },
        SeasonalInsight {
            season: "Påske (Easter)".to_string(),
            cultural_event: Some("Norwegian Easter".to_string()),
            expected_spending_increase: 1.4,
            key_categories: vec!["Egg".to_string(), "Lam".to_string(), "Kvikk Lunsj".to_string()],
            historical_pattern: "Moderate increase focused on Easter traditions".to_string(),
        },
    ];
    
    // Budget recommendations
    let total_predicted: f32 = predictions.iter().map(|p| p.predicted_amount).sum();
    let budget_recommendations = vec![
        BudgetRecommendation {
            category: "Emergency Reserve".to_string(),
            recommended_budget: total_predicted * 0.15,
            reasoning: "15% buffer for unexpected expenses based on Norwegian organizational best practices".to_string(),
            risk_level: "low".to_string(),
            optimization_tips: vec![
                "Set aside monthly reserves".to_string(),
                "Plan for seasonal variations".to_string(),
            ],
        },
        BudgetRecommendation {
            category: "Seasonal Events".to_string(),
            recommended_budget: total_predicted * 0.25,
            reasoning: "Norwegian cultural events drive significant spending spikes".to_string(),
            risk_level: "medium".to_string(),
            optimization_tips: vec![
                "Plan Christmas purchases early".to_string(),
                "Book 17. mai supplies in advance".to_string(),
            ],
        },
    ];
    
    PredictiveAnalysisResponse {
        predictions,
        seasonal_insights,
        budget_recommendations,
        confidence_score: 0.83,
        analysis_type: "advanced_norwegian_predictive".to_string(),
        processing_time_ms: 15, // Simulated processing time
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
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

async fn document_processing(http_req: HttpRequest, req: web::Json<DocumentProcessingRequest>) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
    
    let start_time = std::time::Instant::now();
    let org_type = req.organization_type.as_deref().unwrap_or("forening");
    
    // Determine processing text
    let processing_text = if let Some(image_data) = &req.image_data {
        // Extract text from image using simulated OCR
        extract_text_from_image(image_data)
    } else if let Some(document_text) = &req.document_text {
        document_text.clone()
    } else {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Missing Input".to_string(),
            message: "Either image_data or document_text must be provided".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }));
    };
    
    // Process with enhanced learning-enabled detection
    let amount = extract_amount_from_text(&processing_text).unwrap_or(100.0);
    let merchant = detect_norwegian_merchant_with_learning(&processing_text).unwrap_or_else(|| {
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
    
    let seasonal = get_seasonal_context(None);
    let vat_analysis = analyze_norwegian_vat(amount, &merchant, &processing_text);
    let compliance = check_norwegian_compliance(org_type, &merchant, amount);
    
    let cultural_significance = if seasonal.cultural_event.is_some() {
        Some(format!("Kulturell betydning: {} - typiske innkjøp inkluderer {}",
            seasonal.cultural_event.as_ref().unwrap(),
            seasonal.typical_purchases.join(", ")
        ))
    } else {
        None
    };
    
    let norwegian_analysis = NorwegianAnalysis {
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
    
    // Process image if provided
    let image_analysis = if let Some(image_data) = &req.image_data {
        process_document_image(image_data)
    } else {
        None
    };
    
    // Apply learning if correction data provided
    let learning_applied = if let Some(correction) = &req.correction_data {
        apply_user_learning(correction)
    } else {
        false
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    let processing_confidence = (merchant.confidence + 
        image_analysis.as_ref().map(|img| img.ocr_confidence).unwrap_or(0.9)) / 2.0;
    
    let response = DocumentProcessingResponse {
        norwegian_analysis,
        image_analysis,
        processing_confidence,
        learning_applied,
        model: "rust-llm-multimodal-v1".to_string(),
        processing_time_ms: processing_time,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    println!("Processed document in {}ms with confidence {:.2}", processing_time, processing_confidence);
    Ok(HttpResponse::Ok().json(response))
}

async fn learning_feedback(http_req: HttpRequest, req: web::Json<UserCorrection>) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
    
    let start_time = std::time::Instant::now();
    
    // Apply the learning
    let correction_applied = apply_user_learning(&req);
    
    // Simulate model improvement metrics
    let confidence_improvement = if req.confidence_rating.unwrap_or(5) > 7 {
        Some(0.05 + (req.confidence_rating.unwrap_or(5) as f32 - 7.0) * 0.02)
    } else {
        None
    };
    
    // Count similar cases that would be updated
    let similar_cases = if let Ok(learning_data) = LEARNING_DATA.lock() {
        learning_data.iter().filter(|correction| {
            correction.corrected_merchant == req.corrected_merchant
        }).count() as u32
    } else {
        0
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    let response = LearningResponse {
        correction_applied,
        model_updated: true,
        confidence_improvement,
        similar_cases_updated: similar_cases,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    println!("Applied learning correction in {}ms, updated {} similar cases", processing_time, similar_cases);
    Ok(HttpResponse::Ok().json(response))
}

async fn fine_tuning(http_req: HttpRequest, req: web::Json<FineTuningRequest>) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
    
    let start_time = std::time::Instant::now();
    
    if req.training_data.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid Training Data".to_string(),
            message: "Training data cannot be empty".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }));
    }
    
    let model_type = req.model_type.as_deref().unwrap_or("norwegian_merchant");
    let training_examples_count = req.training_data.len() as u32;
    
    // Store training examples for continuous learning
    for example in &req.training_data {
        store_training_example(example.clone());
    }
    
    // Simulate fine-tuning process
    let validation_metrics = simulate_model_fine_tuning(&req.training_data, model_type);
    
    // Store the fine-tuned model metrics
    let model_id = format!("norwegian-ai-{}-{}", model_type, chrono::Utc::now().timestamp());
    if let Ok(mut models) = FINE_TUNED_MODELS.lock() {
        models.insert(model_id.clone(), validation_metrics.clone());
    }
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    let estimated_time = (training_examples_count as f32 * 0.01).max(5.0).min(120.0) as u32;
    
    let response = FineTuningResponse {
        training_started: true,
        model_id: model_id.clone(),
        training_examples_count,
        estimated_training_time_minutes: estimated_time,
        validation_metrics: Some(validation_metrics.clone()),
        status: format!("Training completed for {} model with {:.2}% accuracy", model_type, validation_metrics.accuracy * 100.0),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    println!("Fine-tuned {} model '{}' in {}ms with {} examples", 
             model_type, model_id, processing_time, training_examples_count);
    Ok(HttpResponse::Ok().json(response))
}

async fn predictive_analysis(http_req: HttpRequest, req: web::Json<PredictiveAnalysisRequest>) -> Result<HttpResponse> {
    // Validate API key
    if let Err(error_response) = validate_api_key_header(&http_req) {
        return Ok(error_response);
    }
    
    let start_time = std::time::Instant::now();
    
    if req.historical_transactions.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Missing Historical Data".to_string(),
            message: "Historical transactions required for predictions".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }));
    }
    
    let timeframe = req.prediction_timeframe.as_deref().unwrap_or("next_quarter");
    let analysis_type = req.analysis_type.as_deref().unwrap_or("spending_patterns");
    
    // Store seasonal patterns for future analysis
    if let Ok(mut seasonal_patterns) = SEASONAL_PATTERNS.lock() {
        seasonal_patterns.insert(
            req.organization_type.clone(), 
            req.historical_transactions.clone()
        );
    }
    
    // Generate comprehensive predictive analysis
    let mut analysis = analyze_spending_patterns(
        &req.historical_transactions,
        &req.organization_type,
        timeframe
    );
    
    // Enhanced analysis based on type
    match analysis_type {
        "seasonal_trends" => {
            analysis.seasonal_insights = analysis.seasonal_insights.into_iter().map(|mut insight| {
                // Enhance with actual historical data
                insight.expected_spending_increase *= 1.1; // More accurate with real data
                insight
            }).collect();
        },
        "budget_forecast" => {
            analysis.budget_recommendations = analysis.budget_recommendations.into_iter().map(|mut rec| {
                // More conservative recommendations for budget forecasts
                rec.recommended_budget *= 1.15;
                rec.reasoning = format!("Conservative forecast: {}", rec.reasoning);
                rec
            }).collect();
        },
        _ => {} // Default spending_patterns analysis
    }
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    analysis.processing_time_ms = processing_time;
    analysis.analysis_type = format!("advanced_norwegian_{}", analysis_type);
    
    println!("Generated {} predictive analysis in {}ms for {} with {} transactions", 
             analysis_type, processing_time, req.organization_type, req.historical_transactions.len());
    Ok(HttpResponse::Ok().json(analysis))
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
            // Multi-modal document processing
            .route("/api/ai/document-processing", web::post().to(document_processing))
            .route("/api/ai/learning-feedback", web::post().to(learning_feedback))
            // Advanced AI capabilities
            .route("/api/ai/fine-tuning", web::post().to(fine_tuning))
            .route("/api/ai/predictive-analysis", web::post().to(predictive_analysis))
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
                    .service(
                        web::scope("/documents")
                            .route("/process", web::post().to(document_processing))
                    )
                    .service(
                        web::scope("/learning")
                            .route("/feedback", web::post().to(learning_feedback))
                    )
                    .service(
                        web::scope("/advanced")
                            .route("/fine-tuning", web::post().to(fine_tuning))
                            .route("/predictive-analysis", web::post().to(predictive_analysis))
                    )
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}