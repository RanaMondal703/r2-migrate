use anyhow::{bail, Context, Result};

pub struct S3Connection {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
}

pub struct Config {
    pub r2: S3Connection,
    pub destination: S3Connection,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_values(
            &required_env("R2_ACCOUNT_ID")?,
            std::env::var("R2_ENDPOINT").ok(),
            &required_env("R2_ACCESS_KEY")?,
            &required_env("R2_SECRET_KEY")?,
            std::env::var("R2_BUCKET").ok(),
            &required_env("S3_ENDPOINT")?,
            &required_env("S3_ACCESS_KEY")?,
            &required_env("S3_SECRET_KEY")?,
            std::env::var("S3_BUCKET").ok(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_values(
        account_id: &str,
        r2_endpoint_override: Option<String>,
        r2_access_key: &str,
        r2_secret_key: &str,
        r2_bucket: Option<String>,
        destination_endpoint: &str,
        destination_access_key: &str,
        destination_secret_key: &str,
        destination_bucket: Option<String>,
    ) -> Result<Self> {
        if account_id.trim().is_empty() {
            bail!("R2_ACCOUNT_ID must not be empty");
        }

        let endpoint = destination_endpoint.trim();
        if endpoint.is_empty() || endpoint == "s3-end-point-api-url" {
            bail!("S3_ENDPOINT must contain the destination API URL");
        }
        if !(endpoint.starts_with("https://") || endpoint.starts_with("http://")) {
            bail!("S3_ENDPOINT must start with http:// or https://");
        }

        let r2_endpoint = match r2_endpoint_override.filter(|value| !value.trim().is_empty()) {
            Some(value) => value.trim().trim_end_matches('/').to_owned(),
            None => format!("https://{}.r2.cloudflarestorage.com", account_id.trim()),
        };

        Ok(Self {
            r2: S3Connection {
                endpoint: r2_endpoint,
                access_key: r2_access_key.to_owned(),
                secret_key: r2_secret_key.to_owned(),
                bucket: bucket_or_default(r2_bucket),
                region: "auto".into(),
            },
            destination: S3Connection {
                endpoint: endpoint.trim_end_matches('/').to_owned(),
                access_key: destination_access_key.to_owned(),
                secret_key: destination_secret_key.to_owned(),
                bucket: bucket_or_default(destination_bucket),
                region: "us-east-1".into(),
            },
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    if value.trim().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

fn bucket_or_default(bucket: Option<String>) -> String {
    bucket.filter(|value| !value.trim().is_empty()).unwrap_or_else(|| "aci".into())
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn defaults_both_buckets_to_aci() {
        let config = Config::from_values(
            "account",
            None,
            "r2-key",
            "r2-secret",
            None,
            "https://minio.example.com",
            "s3-key",
            "s3-secret",
            None,
        )
        .unwrap();

        assert_eq!(config.r2.bucket, "aci");
        assert_eq!(config.destination.bucket, "aci");
    }

    #[test]
    fn rejects_the_destination_endpoint_placeholder() {
        let result = Config::from_values(
            "account",
            None,
            "r2-key",
            "r2-secret",
            None,
            "s3-end-point-api-url",
            "s3-key",
            "s3-secret",
            None,
        );

        assert!(matches!(result, Err(error) if error.to_string().contains("S3_ENDPOINT")));
    }

    #[test]
    fn r2_endpoint_override_replaces_the_cloudflare_endpoint() {
        let config = Config::from_values(
            "account",
            Some("http://127.0.0.1:9000".to_owned()),
            "r2-key",
            "r2-secret",
            None,
            "https://minio.example.com",
            "s3-key",
            "s3-secret",
            None,
        )
        .unwrap();

        assert_eq!(config.r2.endpoint, "http://127.0.0.1:9000");
    }

    #[test]
    fn empty_r2_endpoint_override_falls_back_to_cloudflare() {
        let config = Config::from_values(
            "account",
            Some("   ".to_owned()),
            "r2-key",
            "r2-secret",
            None,
            "https://minio.example.com",
            "s3-key",
            "s3-secret",
            None,
        )
        .unwrap();

        assert_eq!(config.r2.endpoint, "https://account.r2.cloudflarestorage.com");
    }
}
