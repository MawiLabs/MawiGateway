use async_trait::async_trait;
use std::collections::HashSet;

#[async_trait]
pub trait LicenseProvider: Send + Sync {
    /// Check if a specific feature is enabled
    async fn is_feature_enabled(&self, feature: &str) -> bool;
    
    /// Get list of all enabled features
    async fn get_enabled_features(&self) -> HashSet<String>;

    /// Reload license from source
    async fn reload(&self) -> Result<(), String>;
}

/// Default OSS License Provider (No features enabled)
pub struct OssLicenseProvider;

#[async_trait]
impl LicenseProvider for OssLicenseProvider {
    async fn is_feature_enabled(&self, _feature: &str) -> bool {
        false
    }
    
    async fn get_enabled_features(&self) -> HashSet<String> {
        HashSet::new()
    }

    async fn reload(&self) -> Result<(), String> {
        Ok(())
    }
}
