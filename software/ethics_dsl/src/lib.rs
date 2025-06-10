//! ARK Ethics Domain-Specific Language
//! "Your word is a lamp to my feet and a light to my path" - Psalm 119:105
//! 
//! This library implements the core ethics engine for ARK, based on Biblical morality.
//! It provides a domain-specific language for defining, parsing, and evaluating 
//! moral principles according to Scripture.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod ast;
pub mod biblical;
pub mod engine;
pub mod formal;
pub mod grammar;
pub mod interpreter;
pub mod parser;
pub mod semantic;
pub mod types;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub use ast::*;
pub use engine::EthicsEngine;
pub use types::*;

/// Version of the Ethics DSL
pub const DSL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Core Biblical moral principles embedded in the system
pub const CORE_PRINCIPLES: &[(&str, &str)] = &[
    ("SANCTITY_OF_LIFE", "Genesis 1:27 - Created in God's image"),
    ("TRUTH_OVER_LIES", "John 8:44 - Satan is the father of lies"),
    ("PROTECTING_CHILDREN", "Matthew 18:6 - Millstone warning"),
    ("REJECTING_IDOLATRY", "Exodus 20:3 - No other gods"),
    ("SEXUAL_PURITY", "Genesis 1:27, Matthew 19:4-6 - God's design"),
    ("RIGHTEOUSNESS", "Proverbs 21:3 - To do righteousness and justice"),
    ("LOVE_OF_NEIGHBOR", "Mark 12:31 - Love your neighbor as yourself"),
    ("WISDOM_SEEKING", "Proverbs 1:7 - Fear of the Lord is beginning of knowledge"),
];

/// Ethics DSL evaluation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EthicsDecision {
    /// Allow the action/content
    Allow {
        /// Confidence score (0.0 to 1.0)
        confidence: f64,
        /// Biblical justification
        justification: String,
        /// Supporting scripture references
        scripture_refs: Vec<String>,
    },
    /// Deny the action/content
    Deny {
        /// Confidence score (0.0 to 1.0)
        confidence: f64,
        /// Moral violation description
        violation: String,
        /// Violated principles
        violated_principles: Vec<String>,
        /// Supporting scripture references
        scripture_refs: Vec<String>,
    },
    /// Purge the content immediately
    Purge {
        /// Severity level (1-10)
        severity: u8,
        /// Reason for purging
        reason: String,
        /// Violated principles
        violated_principles: Vec<String>,
        /// Supporting scripture references
        scripture_refs: Vec<String>,
    },
}

/// Event to be evaluated by the ethics engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicsEvent {
    /// Unique event identifier
    pub event_id: String,
    /// Actor information
    pub actor: Actor,
    /// Content being evaluated
    pub content: Option<Content>,
    /// Context information
    pub context: Context,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Actor in an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Actor type
    pub actor_type: ActorType,
    /// Actor tags/classifications
    pub tags: Vec<String>,
    /// Trust level (0.0 to 1.0)
    pub trust_level: f64,
    /// History of previous evaluations
    pub history: Option<ActorHistory>,
}

/// Type of actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorType {
    /// Human person
    Person,
    /// AI/Algorithm
    ArtificialIntelligence,
    /// Content (text, image, video)
    Content,
    /// Institution/Organization
    Institution,
    /// Elite/Authority figure
    Elite,
}

/// Content being evaluated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Content type
    pub content_type: ContentType,
    /// Content data/text
    pub data: String,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Content hash for integrity
    pub content_hash: String,
}

/// Type of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    /// Text content
    Text,
    /// Image content
    Image,
    /// Video content
    Video,
    /// Audio content
    Audio,
    /// Code/Software
    Code,
    /// Educational material
    Educational,
    /// News/Information
    News,
    /// Entertainment
    Entertainment,
}

/// Context for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Geographic location (ISO 3166)
    pub location: Option<String>,
    /// Cultural context
    pub culture: Option<String>,
    /// Platform/medium
    pub platform: Option<String>,
    /// Audience information
    pub audience: Option<Audience>,
    /// Time sensitivity
    pub urgency: UrgencyLevel,
}

/// Audience information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audience {
    /// Age groups present
    pub age_groups: Vec<AgeGroup>,
    /// Vulnerable populations
    pub vulnerable_groups: Vec<String>,
    /// Size of audience
    pub size: Option<u64>,
}

/// Age group classifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgeGroup {
    /// Children (0-12)
    Children,
    /// Teenagers (13-17)
    Teenagers,
    /// YoungAdults (18-25)
    YoungAdults,
    /// Adults (26-64)
    Adults,
    /// Seniors (65+)
    Seniors,
}

/// Urgency level for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical/Emergency
    Critical,
}

/// Actor history for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorHistory {
    /// Previous violations
    pub violations: Vec<Violation>,
    /// Trust score history
    pub trust_history: Vec<TrustEntry>,
    /// Total evaluations
    pub total_evaluations: u64,
}

/// Moral violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Timestamp of violation
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Violated principle
    pub principle: String,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Trust score entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEntry {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Trust score (0.0 to 1.0)
    pub score: f64,
    /// Reason for change
    pub reason: String,
}

/// Errors that can occur in the ethics system
#[derive(Error, Debug)]
pub enum EthicsError {
    /// Parsing error
    #[error("Parsing error: {0}")]
    ParseError(String),
    
    /// Evaluation error
    #[error("Evaluation error: {0}")]
    EvaluationError(String),
    
    /// Biblical reference error
    #[error("Biblical reference error: {0}")]
    BiblicalReferenceError(String),
    
    /// Formal verification error
    #[error("Formal verification error: {0}")]
    FormalVerificationError(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    /// Runtime error
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

/// Result type for ethics operations
pub type EthicsResult<T> = Result<T, EthicsError>;

/// Main ethics evaluator interface
pub trait EthicsEvaluator {
    /// Evaluate an event and return a decision
    fn evaluate(&self, event: &EthicsEvent) -> EthicsResult<EthicsDecision>;
    
    /// Validate DSL rules
    fn validate_rules(&self, rules: &str) -> EthicsResult<()>;
    
    /// Update moral rules
    fn update_rules(&mut self, rules: &str) -> EthicsResult<()>;
    
    /// Get current moral foundation
    fn get_foundation(&self) -> &biblical::BiblicalFoundation;
}

/// Configuration for the ethics engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicsConfig {
    /// Enable formal verification
    pub formal_verification: bool,
    /// Strictness level (1-10)
    pub strictness_level: u8,
    /// Language preference
    pub language: String,
    /// Cultural adaptations
    pub cultural_adaptations: Vec<String>,
    /// Performance settings
    pub performance: PerformanceConfig,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum evaluation time (milliseconds)
    pub max_evaluation_time_ms: u64,
    /// Enable parallel processing
    pub parallel_processing: bool,
    /// Cache size
    pub cache_size: usize,
    /// Memory limit (MB)
    pub memory_limit_mb: usize,
}

impl Default for EthicsConfig {
    fn default() -> Self {
        Self {
            formal_verification: true,
            strictness_level: 8,
            language: "en".to_string(),
            cultural_adaptations: vec!["western".to_string()],
            performance: PerformanceConfig {
                max_evaluation_time_ms: 50,
                parallel_processing: true,
                cache_size: 10000,
                memory_limit_mb: 512,
            },
        }
    }
}

/// Biblical moral tags for classification
pub mod tags {
    /// Content promoting idolatry
    pub const IDOLATRY: &str = "IDOLATRY";
    /// LGBT propaganda
    pub const LGBT_PROP: &str = "LGBT_PROP";
    /// Sexual immorality
    pub const SEXUAL_IMMORALITY: &str = "SEXUAL_IMMORALITY";
    /// Violence against innocent
    pub const VIOLENCE_INNOCENT: &str = "VIOLENCE_INNOCENT";
    /// Blasphemy
    pub const BLASPHEMY: &str = "BLASPHEMY";
    /// Deception/lies
    pub const DECEPTION: &str = "DECEPTION";
    /// Corruption of children
    pub const CHILD_CORRUPTION: &str = "CHILD_CORRUPTION";
    /// Materialism/greed
    pub const MATERIALISM: &str = "MATERIALISM";
    /// Pride/arrogance
    pub const PRIDE: &str = "PRIDE";
    /// Occultism
    pub const OCCULTISM: &str = "OCCULTISM";
    
    /// All moral violation tags
    pub const ALL_VIOLATION_TAGS: &[&str] = &[
        IDOLATRY,
        LGBT_PROP,
        SEXUAL_IMMORALITY,
        VIOLENCE_INNOCENT,
        BLASPHEMY,
        DECEPTION,
        CHILD_CORRUPTION,
        MATERIALISM,
        PRIDE,
        OCCULTISM,
    ];
}

/// Utility functions
pub mod utils {
    use super::*;
    
    /// Create a new ethics event
    pub fn create_event(
        event_id: String,
        actor: Actor,
        content: Option<Content>,
        context: Context,
    ) -> EthicsEvent {
        EthicsEvent {
            event_id,
            actor,
            content,
            context,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Calculate confidence score based on multiple factors
    pub fn calculate_confidence(
        rule_matches: usize,
        context_clarity: f64,
        biblical_support: f64,
    ) -> f64 {
        let base_confidence = (rule_matches as f64) * 0.3;
        let context_weight = context_clarity * 0.3;
        let biblical_weight = biblical_support * 0.4;
        
        (base_confidence + context_weight + biblical_weight).min(1.0)
    }
    
    /// Validate scripture reference format
    pub fn validate_scripture_ref(reference: &str) -> bool {
        // Basic validation - would be more sophisticated in practice
        reference.contains(':') && reference.len() > 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ethics_decision_serialization() {
        let decision = EthicsDecision::Allow {
            confidence: 0.95,
            justification: "Content promotes Biblical values".to_string(),
            scripture_refs: vec!["Matthew 5:16".to_string()],
        };
        
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: EthicsDecision = serde_json::from_str(&json).unwrap();
        
        assert_eq!(decision, deserialized);
    }
    
    #[test]
    fn test_core_principles() {
        assert_eq!(CORE_PRINCIPLES.len(), 8);
        assert!(CORE_PRINCIPLES.iter().any(|(key, _)| key == &"SANCTITY_OF_LIFE"));
    }
    
    #[test]
    fn test_confidence_calculation() {
        let confidence = utils::calculate_confidence(3, 0.8, 0.9);
        assert!(confidence > 0.0 && confidence <= 1.0);
    }
    
    #[test]
    fn test_scripture_reference_validation() {
        assert!(utils::validate_scripture_ref("Genesis 1:27"));
        assert!(utils::validate_scripture_ref("Matthew 18:6"));
        assert!(!utils::validate_scripture_ref("Invalid"));
    }
}