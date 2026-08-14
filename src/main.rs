mod config;

use anyhow::{Context, Result};
use aws_sdk_s3::{
    config::{Builder, Credentials, Region},
    error::ProvideErrorMetadata,
    Client,
};
use config::{Config, S3Connection};

struct TransferSummary {
    copied: u64,
    skipped: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let source = build_client(&config.r2);
    let destination = build_client(&config.destination);
    let summary = copy_all(&source, &destination, &config).await?;

    println!(
        "Done. Copied {} files; skipped {} existing files. R2 was not modified.",
        summary.copied, summary.skipped
    );
    Ok(())
}

fn build_client(connection: &S3Connection) -> Client {
    let credentials = Credentials::new(
        &connection.access_key,
        &connection.secret_key,
        None,
        None,
        "migration",
    );
    let config = Builder::new()
        .region(Region::new(connection.region.clone()))
        .endpoint_url(&connection.endpoint)
        .force_path_style(true)
        .credentials_provider(credentials)
        .behavior_version_latest()
        .build();

    Client::from_conf(config)
}

async fn copy_all(source: &Client, destination: &Client, config: &Config) -> Result<TransferSummary> {
    let mut continuation_token = None;
    let mut summary = TransferSummary {
        copied: 0,
        skipped: 0,
    };

    loop {
        let mut request = source.list_objects_v2().bucket(&config.r2.bucket);
        if let Some(token) = continuation_token.as_deref() {
            request = request.continuation_token(token);
        }
        let page = request.send().await.context("listing R2 objects")?;

        for object in page.contents() {
            let key = object.key().context("R2 returned an object without a key")?;
            let source_size = object.size();

            match destination
                .head_object()
                .bucket(&config.destination.bucket)
                .key(key)
                .send()
                .await
            {
                Ok(existing) if should_skip(source_size, existing.content_length()) => {
                    println!("Skipping existing: {key}");
                    summary.skipped += 1;
                    continue;
                }
                Ok(_) => {}
                Err(error) if is_not_found(&error) => {}
                Err(error) => return Err(error).context(format!("checking destination object {key}")),
            }

            println!("Copying: {key}");
            let object = source
                .get_object()
                .bucket(&config.r2.bucket)
                .key(key)
                .send()
                .await
                .with_context(|| format!("downloading R2 object {key}"))?;

            let content_type = object.content_type().map(str::to_owned);
            let mut put = destination
                .put_object()
                .bucket(&config.destination.bucket)
                .key(key)
                .body(object.body);
            if let Some(content_type) = content_type {
                put = put.content_type(content_type);
            }
            put.send()
                .await
                .with_context(|| format!("uploading destination object {key}"))?;
            summary.copied += 1;
        }

        if page.is_truncated().unwrap_or(false) {
            continuation_token = page.next_continuation_token().map(str::to_owned);
        } else {
            break;
        }
    }

    Ok(summary)
}

fn should_skip(source_size: Option<i64>, destination_size: Option<i64>) -> bool {
    source_size.is_some() && source_size == destination_size
}

fn is_not_found<E: ProvideErrorMetadata>(error: &E) -> bool {
    error.code() == Some("NotFound") || error.code() == Some("NoSuchKey")
}

#[cfg(test)]
mod tests {
    use super::should_skip;

    #[test]
    fn skips_only_when_destination_size_matches_source() {
        assert!(should_skip(Some(42), Some(42)));
        assert!(!should_skip(Some(42), Some(41)));
        assert!(!should_skip(Some(42), None));
    }
}
