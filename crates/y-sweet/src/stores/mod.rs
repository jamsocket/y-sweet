pub mod filesystem;
pub mod gcs;
pub mod s3;

#[derive(Debug, Clone, PartialEq)]
pub enum StoreTarget {
    S3 { bucket: String, prefix: Option<String> },
    Gcs { bucket: String, prefix: Option<String> },
    Filesystem(String),
}

fn parse_bucket_url(url_str: &str) -> anyhow::Result<(String, Option<String>)> {
    let url = url::Url::parse(url_str)?;
    let bucket = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid store URL, no bucket: {}", url_str))?
        .to_owned();
    let prefix = url.path().trim_matches('/').to_owned();
    let prefix = (!prefix.is_empty()).then_some(prefix);
    Ok((bucket, prefix))
}

pub fn parse_store_target(store_path: &str) -> anyhow::Result<StoreTarget> {
    if store_path.starts_with("s3://") {
        let (bucket, prefix) = parse_bucket_url(store_path)?;
        Ok(StoreTarget::S3 { bucket, prefix })
    } else if store_path.starts_with("gs://") {
        let (bucket, prefix) = parse_bucket_url(store_path)?;
        Ok(StoreTarget::Gcs { bucket, prefix })
    } else {
        Ok(StoreTarget::Filesystem(store_path.to_string()))
    }
}

pub fn target_from_bucket_env(bucket: &str, prefix: Option<String>) -> anyhow::Result<StoreTarget> {
    if bucket.starts_with("s3://") || bucket.starts_with("gs://") {
        let target = parse_store_target(bucket)?;
        match (target, prefix) {
            (StoreTarget::S3 { bucket, prefix: None }, p) => {
                Ok(StoreTarget::S3 { bucket, prefix: p })
            }
            (StoreTarget::Gcs { bucket, prefix: None }, p) => {
                Ok(StoreTarget::Gcs { bucket, prefix: p })
            }
            _ => anyhow::bail!(
                "STORAGE_BUCKET must not contain a path when used with STORAGE_PREFIX: {}",
                bucket
            ),
        }
    } else {
        Ok(StoreTarget::S3 {
            bucket: bucket.to_string(),
            prefix,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_s3_url_with_prefix() {
        let t = parse_store_target("s3://my-bucket/some/prefix").unwrap();
        assert!(matches!(t, StoreTarget::S3 { ref bucket, ref prefix }
            if bucket == "my-bucket" && prefix.as_deref() == Some("some/prefix")));
    }

    #[test]
    fn parses_s3_url_without_prefix() {
        let t = parse_store_target("s3://my-bucket").unwrap();
        assert!(matches!(t, StoreTarget::S3 { ref bucket, ref prefix }
            if bucket == "my-bucket" && prefix.is_none()));
    }

    #[test]
    fn parses_gs_url_with_prefix() {
        let t = parse_store_target("gs://my-bucket/docs").unwrap();
        assert!(matches!(t, StoreTarget::Gcs { ref bucket, ref prefix }
            if bucket == "my-bucket" && prefix.as_deref() == Some("docs")));
    }

    #[test]
    fn parses_filesystem_path() {
        let t = parse_store_target("/var/data/ysweet").unwrap();
        assert!(matches!(t, StoreTarget::Filesystem(ref p) if p == "/var/data/ysweet"));
    }

    #[test]
    fn trailing_slash_means_no_prefix() {
        let t = parse_store_target("s3://my-bucket/").unwrap();
        assert!(matches!(t, StoreTarget::S3 { ref prefix, .. } if prefix.is_none()));
    }

    #[test]
    fn bucket_env_bare_name_is_s3() {
        let t = target_from_bucket_env("my-bucket", Some("p".to_string())).unwrap();
        assert!(matches!(t, StoreTarget::S3 { ref bucket, ref prefix }
            if bucket == "my-bucket" && prefix.as_deref() == Some("p")));
    }

    #[test]
    fn bucket_env_gs_scheme_is_gcs() {
        let t = target_from_bucket_env("gs://my-bucket", None).unwrap();
        assert!(matches!(t, StoreTarget::Gcs { ref bucket, .. } if bucket == "my-bucket"));
    }

    #[test]
    fn bucket_env_s3_scheme_is_s3() {
        let t = target_from_bucket_env("s3://my-bucket", None).unwrap();
        assert!(matches!(t, StoreTarget::S3 { ref bucket, .. } if bucket == "my-bucket"));
    }

    #[test]
    fn s3_url_with_no_bucket_is_an_error() {
        assert!(parse_store_target("s3://").is_err());
    }
}
