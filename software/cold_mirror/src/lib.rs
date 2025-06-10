//! ARK Cold-Mirror - Neural Network Harm Prediction System
//! "But I have prayed for you, that your faith should not fail" - Luke 22:32
//!
//! This library implements the Cold-Mirror harm prediction system that analyzes
//! content and actors to predict potential moral and physical harm using neural networks.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod analysis;
pub mod inference;
pub mod models;
pub mod preprocessing;
pub mod risk_assessment;
pub mod training;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use chrono::{DateTime, Utc};
use ethics_dsl::{EthicsEvent, EthicsDecision};

/// Version of the Cold-Mirror system
pub const COLD_MIRROR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum batch size for inference (performance requirement: ≤50ms for 512 events)
pub const MAX_BATCH_SIZE: usize = 512;

/// Confidence threshold for high-confidence predictions
pub const HIGH_CONFIDENCE_THRESHOLD: f32 = 0.85;

/// Cold-Mirror prediction result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HarmPrediction {
    /// Predicted harm level (0.0 to 1.0)
    pub harm_level: f32,
    /// Confidence in prediction (0.0 to 1.0)
    pub confidence: f32,
    /// Time horizon for prediction (hours)
    pub time_horizon: f32,
    /// Specific harm categories detected
    pub harm_categories: Vec<HarmCategory>,
    /// Risk factors identified
    pub risk_factors: Vec<RiskFactor>,
    /// Recommended action
    pub recommended_action: RecommendedAction,
    /// Prediction timestamp
    pub timestamp: DateTime<Utc>,
    /// Model version used
    pub model_version: String,
}

/// Categories of potential harm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HarmCategory {
    /// Moral degradation
    MoralDegradation {
        /// Specific moral violation
        violation: String,
        /// Severity (0.0 to 1.0)
        severity: f32,
    },
    /// Physical harm to individuals
    PhysicalHarm {
        /// Type of physical harm
        harm_type: String,
        /// Estimated victims
        victim_count: Option<u32>,
        /// Likelihood (0.0 to 1.0)
        likelihood: f32,
    },
    /// Psychological harm
    PsychologicalHarm {
        /// Type of psychological damage
        damage_type: String,
        /// Vulnerable groups affected
        vulnerable_groups: Vec<String>,
        /// Long-term impact assessment
        long_term_impact: f32,
    },
    /// Social harm and degradation
    SocialHarm {
        /// Social structure affected
        structure: String,
        /// Scale of impact
        scale: ImpactScale,
        /// Duration of effect
        duration: EffectDuration,
    },
    /// Spiritual harm
    SpiritualHarm {
        /// Spiritual principle violated
        principle: String,
        /// Biblical reference
        scripture_reference: String,
        /// Eternal consequences assessment
        eternal_impact: f32,
    },
}

/// Risk factors that contribute to harm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskFactor {
    /// Factor name
    pub name: String,
    /// Weight/importance (0.0 to 1.0)
    pub weight: f32,
    /// Description
    pub description: String,
    /// Evidence supporting this factor
    pub evidence: Vec<String>,
}

/// Recommended actions based on prediction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendedAction {
    /// Allow with monitoring
    AllowWithMonitoring {
        /// Monitoring level
        monitoring_level: MonitoringLevel,
        /// Review interval (hours)
        review_interval: f32,
    },
    /// Block immediately
    Block {
        /// Reason for blocking
        reason: String,
        /// Duration of block (hours, None = permanent)
        duration: Option<f32>,
    },
    /// Quarantine for review
    Quarantine {
        /// Review priority
        priority: ReviewPriority,
        /// Maximum quarantine time (hours)
        max_duration: f32,
    },
    /// Purge immediately
    Purge {
        /// Urgency level
        urgency: UrgencyLevel,
        /// Escalation required
        escalate: bool,
    },
}

/// Scale of impact
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactScale {
    /// Individual impact
    Individual,
    /// Family/small group impact
    Family,
    /// Community impact
    Community,
    /// Regional impact
    Regional,
    /// National impact
    National,
    /// Global impact
    Global,
}

/// Duration of harmful effects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffectDuration {
    /// Temporary effect (< 1 day)
    Temporary,
    /// Short-term effect (1-30 days)
    ShortTerm,
    /// Medium-term effect (1-12 months)
    MediumTerm,
    /// Long-term effect (> 1 year)
    LongTerm,
    /// Permanent effect
    Permanent,
}

/// Monitoring levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MonitoringLevel {
    /// Basic monitoring
    Basic,
    /// Enhanced monitoring
    Enhanced,
    /// Intensive monitoring
    Intensive,
    /// Real-time monitoring
    RealTime,
}

/// Review priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReviewPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Urgency levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UrgencyLevel {
    /// Low urgency
    Low,
    /// Medium urgency
    Medium,
    /// High urgency
    High,
    /// Critical urgency
    Critical,
}

/// Input data for harm prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionInput {
    /// Ethics event to analyze
    pub event: EthicsEvent,
    /// Additional context data
    pub context: PredictionContext,
    /// Historical data if available
    pub history: Option<HistoricalData>,
}

/// Context for prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionContext {
    /// Current time
    pub timestamp: DateTime<Utc>,
    /// Geographic context
    pub location: Option<GeographicContext>,
    /// Social context
    pub social_context: Option<SocialContext>,
    /// Economic context
    pub economic_context: Option<EconomicContext>,
    /// Political context
    pub political_context: Option<PoliticalContext>,
}

/// Geographic context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicContext {
    /// Country code (ISO 3166)
    pub country: String,
    /// Region/state
    pub region: Option<String>,
    /// City
    pub city: Option<String>,
    /// Cultural indicators
    pub cultural_indicators: Vec<String>,
}

/// Social context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialContext {
    /// Social media platform
    pub platform: Option<String>,
    /// Community type
    pub community_type: Option<String>,
    /// Social dynamics indicators
    pub dynamics: Vec<String>,
}

/// Economic context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicContext {
    /// Economic indicators
    pub indicators: HashMap<String, f32>,
    /// Market conditions
    pub market_conditions: Vec<String>,
}

/// Political context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalContext {
    /// Political climate indicators
    pub climate_indicators: Vec<String>,
    /// Stability measures
    pub stability: f32,
}

/// Historical data for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalData {
    /// Previous events from same actor
    pub actor_history: Vec<EthicsEvent>,
    /// Previous predictions
    pub prediction_history: Vec<HarmPrediction>,
    /// Outcome data
    pub outcomes: Vec<OutcomeData>,
}

/// Outcome data for model improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeData {
    /// Original prediction
    pub prediction: HarmPrediction,
    /// Actual outcome observed
    pub actual_outcome: ActualOutcome,
    /// Time between prediction and outcome
    pub time_to_outcome: f32,
    /// Accuracy metrics
    pub accuracy_metrics: AccuracyMetrics,
}

/// Actual observed outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualOutcome {
    /// Whether harm occurred
    pub harm_occurred: bool,
    /// Actual harm level (0.0 to 1.0)
    pub actual_harm_level: f32,
    /// Categories of harm that occurred
    pub harm_categories: Vec<HarmCategory>,
    /// Description of what happened
    pub description: String,
}

/// Accuracy metrics for model evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyMetrics {
    /// Prediction accuracy (0.0 to 1.0)
    pub accuracy: f32,
    /// Precision (true positives / (true positives + false positives))
    pub precision: f32,
    /// Recall (true positives / (true positives + false negatives))
    pub recall: f32,
    /// F1 score
    pub f1_score: f32,
    /// Mean absolute error
    pub mae: f32,
}

/// Cold-Mirror errors
#[derive(Error, Debug)]
pub enum ColdMirrorError {
    /// Model loading error
    #[error("Model loading error: {0}")]
    ModelLoadError(String),
    
    /// Inference error
    #[error("Inference error: {0}")]
    InferenceError(String),
    
    /// Preprocessing error
    #[error("Preprocessing error: {0}")]
    PreprocessingError(String),
    
    /// Timeout error
    #[error("Timeout error: operation took too long")]
    TimeoutError,
    
    /// Resource error
    #[error("Resource error: {0}")]
    ResourceError(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    /// Data error
    #[error("Data error: {0}")]
    DataError(String),
}

/// Result type for Cold-Mirror operations
pub type ColdMirrorResult<T> = Result<T, ColdMirrorError>;

/// Main Cold-Mirror prediction interface
pub trait HarmPredictor {
    /// Predict harm for a single event
    fn predict_harm(&self, input: &PredictionInput) -> ColdMirrorResult<HarmPrediction>;
    
    /// Predict harm for a batch of events (performance optimized)
    fn predict_harm_batch(&self, inputs: &[PredictionInput]) -> ColdMirrorResult<Vec<HarmPrediction>>;
    
    /// Update model with new outcome data
    fn update_with_outcome(&mut self, outcome: &OutcomeData) -> ColdMirrorResult<()>;
    
    /// Get model performance metrics
    fn get_performance_metrics(&self) -> ColdMirrorResult<ModelMetrics>;
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    /// Overall accuracy
    pub accuracy: f32,
    /// Precision by harm category
    pub precision_by_category: HashMap<String, f32>,
    /// Recall by harm category
    pub recall_by_category: HashMap<String, f32>,
    /// Average inference time (milliseconds)
    pub avg_inference_time_ms: f32,
    /// Total predictions made
    pub total_predictions: u64,
    /// Model version
    pub model_version: String,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

/// Configuration for Cold-Mirror system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdMirrorConfig {
    /// Model configuration
    pub model_config: ModelConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Logging settings
    pub logging: LoggingConfig,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model file path
    pub model_path: String,
    /// Model type
    pub model_type: ModelType,
    /// Input preprocessing settings
    pub preprocessing: PreprocessingConfig,
    /// Output postprocessing settings
    pub postprocessing: PostprocessingConfig,
}

/// Model types supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    /// Transformer-based model
    Transformer,
    /// Convolutional neural network
    CNN,
    /// Recurrent neural network
    RNN,
    /// Hybrid model
    Hybrid,
    /// Ensemble model
    Ensemble,
}

/// Preprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    /// Text preprocessing settings
    pub text: TextPreprocessingConfig,
    /// Image preprocessing settings
    pub image: ImagePreprocessingConfig,
    /// Audio preprocessing settings
    pub audio: AudioPreprocessingConfig,
}

/// Text preprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPreprocessingConfig {
    /// Maximum sequence length
    pub max_length: usize,
    /// Tokenizer settings
    pub tokenizer: TokenizerConfig,
    /// Normalization settings
    pub normalization: NormalizationConfig,
}

/// Tokenizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerConfig {
    /// Tokenizer type
    pub tokenizer_type: String,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Special tokens
    pub special_tokens: HashMap<String, String>,
}

/// Text normalization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    /// Lowercase text
    pub lowercase: bool,
    /// Remove punctuation
    pub remove_punctuation: bool,
    /// Remove stop words
    pub remove_stop_words: bool,
    /// Unicode normalization
    pub unicode_normalization: String,
}

/// Image preprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePreprocessingConfig {
    /// Target image size
    pub target_size: (u32, u32),
    /// Normalization mean
    pub mean: [f32; 3],
    /// Normalization standard deviation
    pub std: [f32; 3],
}

/// Audio preprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPreprocessingConfig {
    /// Sample rate
    pub sample_rate: u32,
    /// Duration (seconds)
    pub duration: f32,
    /// Feature extraction method
    pub feature_extraction: String,
}

/// Postprocessing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostprocessingConfig {
    /// Confidence calibration
    pub calibration: CalibrationConfig,
    /// Output filtering
    pub filtering: FilteringConfig,
}

/// Confidence calibration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    /// Calibration method
    pub method: String,
    /// Calibration parameters
    pub parameters: HashMap<String, f32>,
}

/// Output filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteringConfig {
    /// Minimum confidence threshold
    pub min_confidence: f32,
    /// Maximum predictions per batch
    pub max_predictions: usize,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Timeout for inference (milliseconds)
    pub inference_timeout_ms: u64,
    /// Number of worker threads
    pub num_threads: usize,
    /// Memory limit (MB)
    pub memory_limit_mb: usize,
    /// GPU acceleration settings
    pub gpu_acceleration: Option<GpuConfig>,
}

/// GPU acceleration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// Device ID
    pub device_id: u32,
    /// Memory allocation strategy
    pub memory_strategy: String,
    /// Precision (fp16, fp32)
    pub precision: String,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Model integrity verification
    pub verify_model_integrity: bool,
    /// Input sanitization
    pub sanitize_inputs: bool,
    /// Side-channel protection
    pub side_channel_protection: bool,
    /// Differential privacy
    pub differential_privacy: Option<DifferentialPrivacyConfig>,
}

/// Differential privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialPrivacyConfig {
    /// Privacy budget (epsilon)
    pub epsilon: f32,
    /// Noise mechanism
    pub noise_mechanism: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log predictions
    pub log_predictions: bool,
    /// Log performance metrics
    pub log_metrics: bool,
    /// Log file path
    pub log_file: Option<String>,
}

impl Default for ColdMirrorConfig {
    fn default() -> Self {
        Self {
            model_config: ModelConfig {
                model_path: "models/cold_mirror_v1.safetensors".to_string(),
                model_type: ModelType::Transformer,
                preprocessing: PreprocessingConfig {
                    text: TextPreprocessingConfig {
                        max_length: 512,
                        tokenizer: TokenizerConfig {
                            tokenizer_type: "bert".to_string(),
                            vocab_size: 30000,
                            special_tokens: HashMap::new(),
                        },
                        normalization: NormalizationConfig {
                            lowercase: true,
                            remove_punctuation: false,
                            remove_stop_words: false,
                            unicode_normalization: "NFKC".to_string(),
                        },
                    },
                    image: ImagePreprocessingConfig {
                        target_size: (224, 224),
                        mean: [0.485, 0.456, 0.406],
                        std: [0.229, 0.224, 0.225],
                    },
                    audio: AudioPreprocessingConfig {
                        sample_rate: 16000,
                        duration: 10.0,
                        feature_extraction: "mfcc".to_string(),
                    },
                },
                postprocessing: PostprocessingConfig {
                    calibration: CalibrationConfig {
                        method: "platt".to_string(),
                        parameters: HashMap::new(),
                    },
                    filtering: FilteringConfig {
                        min_confidence: 0.1,
                        max_predictions: 1000,
                    },
                },
            },
            performance: PerformanceConfig {
                max_batch_size: MAX_BATCH_SIZE,
                inference_timeout_ms: 50, // 50ms requirement
                num_threads: 4,
                memory_limit_mb: 2048,
                gpu_acceleration: None,
            },
            security: SecurityConfig {
                verify_model_integrity: true,
                sanitize_inputs: true,
                side_channel_protection: true,
                differential_privacy: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                log_predictions: true,
                log_metrics: true,
                log_file: Some("cold_mirror.log".to_string()),
            },
        }
    }
}

/// Utility functions
pub mod utils {
    use super::*;
    
    /// Create a prediction input from an ethics event
    pub fn create_prediction_input(
        event: EthicsEvent,
        context: Option<PredictionContext>,
        history: Option<HistoricalData>,
    ) -> PredictionInput {
        PredictionInput {
            event,
            context: context.unwrap_or_else(|| PredictionContext {
                timestamp: Utc::now(),
                location: None,
                social_context: None,
                economic_context: None,
                political_context: None,
            }),
            history,
        }
    }
    
    /// Calculate harm score from prediction
    pub fn calculate_harm_score(prediction: &HarmPrediction) -> f32 {
        prediction.harm_level * prediction.confidence
    }
    
    /// Check if prediction meets high confidence threshold
    pub fn is_high_confidence(prediction: &HarmPrediction) -> bool {
        prediction.confidence >= HIGH_CONFIDENCE_THRESHOLD
    }
    
    /// Convert harm prediction to ethics decision
    pub fn to_ethics_decision(prediction: &HarmPrediction) -> EthicsDecision {
        match &prediction.recommended_action {
            RecommendedAction::AllowWithMonitoring { .. } => {
                EthicsDecision::Allow {
                    confidence: prediction.confidence as f64,
                    justification: "Cold-Mirror analysis indicates acceptable risk".to_string(),
                    scripture_refs: vec!["1 Thessalonians 5:21".to_string()],
                }
            }
            RecommendedAction::Block { reason, .. } => {
                EthicsDecision::Deny {
                    confidence: prediction.confidence as f64,
                    violation: reason.clone(),
                    violated_principles: prediction.harm_categories.iter()
                        .map(|cat| format!("{:?}", cat))
                        .collect(),
                    scripture_refs: vec!["Proverbs 27:14".to_string()],
                }
            }
            RecommendedAction::Quarantine { .. } => {
                EthicsDecision::Deny {
                    confidence: prediction.confidence as f64,
                    violation: "Content requires review".to_string(),
                    violated_principles: vec!["CAUTION".to_string()],
                    scripture_refs: vec!["Proverbs 14:15".to_string()],
                }
            }
            RecommendedAction::Purge { .. } => {
                EthicsDecision::Purge {
                    severity: (prediction.harm_level * 10.0) as u8,
                    reason: "High harm risk detected".to_string(),
                    violated_principles: prediction.harm_categories.iter()
                        .map(|cat| format!("{:?}", cat))
                        .collect(),
                    scripture_refs: vec!["Matthew 18:6".to_string()],
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_harm_prediction_serialization() {
        let prediction = HarmPrediction {
            harm_level: 0.75,
            confidence: 0.90,
            time_horizon: 24.0,
            harm_categories: vec![],
            risk_factors: vec![],
            recommended_action: RecommendedAction::Block {
                reason: "Test".to_string(),
                duration: Some(24.0),
            },
            timestamp: Utc::now(),
            model_version: "v1.0".to_string(),
        };
        
        let json = serde_json::to_string(&prediction).unwrap();
        let deserialized: HarmPrediction = serde_json::from_str(&json).unwrap();
        
        assert_eq!(prediction.harm_level, deserialized.harm_level);
        assert_eq!(prediction.confidence, deserialized.confidence);
    }
    
    #[test]
    fn test_harm_score_calculation() {
        let prediction = HarmPrediction {
            harm_level: 0.8,
            confidence: 0.9,
            time_horizon: 24.0,
            harm_categories: vec![],
            risk_factors: vec![],
            recommended_action: RecommendedAction::Block {
                reason: "Test".to_string(),
                duration: Some(24.0),
            },
            timestamp: Utc::now(),
            model_version: "v1.0".to_string(),
        };
        
        let score = utils::calculate_harm_score(&prediction);
        assert_eq!(score, 0.72); // 0.8 * 0.9
    }
    
    #[test]
    fn test_high_confidence_threshold() {
        let high_conf_prediction = HarmPrediction {
            harm_level: 0.5,
            confidence: 0.9,
            time_horizon: 24.0,
            harm_categories: vec![],
            risk_factors: vec![],
            recommended_action: RecommendedAction::Block {
                reason: "Test".to_string(),
                duration: Some(24.0),
            },
            timestamp: Utc::now(),
            model_version: "v1.0".to_string(),
        };
        
        assert!(utils::is_high_confidence(&high_conf_prediction));
        
        let low_conf_prediction = HarmPrediction {
            confidence: 0.7,
            ..high_conf_prediction
        };
        
        assert!(!utils::is_high_confidence(&low_conf_prediction));
    }
} 