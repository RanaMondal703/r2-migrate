# R2 to S3-Compatible Storage Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a resumable CLI that copies every object from R2 bucket `aci` to an S3-compatible destination bucket.

**Architecture:** Use two independently configured AWS S3 SDK clients: R2 is read-only and the S3-compatible destination (RustFS or MinIO) is write-only. List R2 with pagination, skip destination objects with equal content length, and stream each missing object directly into the destination while retaining its key and available content type.

**Tech Stack:** Rust 2024, Tokio, aws-sdk-s3, anyhow.

## Global Constraints

- Source objects in R2 must never be modified or deleted.
- The destination endpoint is supplied by `S3_ENDPOINT`; `s3-end-point-api-url` is only a placeholder and must be replaced before use.
- Source and destination buckets default to `aci`.
- Preserve object keys and content type when R2 provides it.
- A rerun must skip an existing destination object when its content length equals the source object length.

---

### Task 1: Destination configuration

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`
- Test: `src/config.rs`

**Interfaces:**
- Produces: `Config::from_env() -> anyhow::Result<Config>`.
- Produces: `Config { r2: S3Connection, destination: S3Connection }` where `S3Connection` has `endpoint`, `access_key`, `secret_key`, `bucket`, and `region` fields.

- [ ] **Step 1: Write failing configuration tests**

```rust
#[test]
fn defaults_both_buckets_to_aci() {
    let config = Config::from_values("account", "r2-key", "r2-secret", None,
        "https://rustfs.example", "rustfs-key", "rustfs-secret", None).unwrap();
    assert_eq!(config.r2.bucket, "aci");
    assert_eq!(config.rustfs.bucket, "aci");
}

#[test]
fn rejects_the_destination_endpoint_placeholder() {
    let result = Config::from_values("account", "r2-key", "r2-secret", None,
        "rust-fs-end-point-api-url", "rustfs-key", "rustfs-secret", None);
    assert!(result.unwrap_err().to_string().contains("S3_ENDPOINT"));
}
```

- [ ] **Step 2: Run the tests to verify failure**

Run: `cargo test config`

Expected: FAIL because `Config` does not exist.

- [ ] **Step 3: Implement the configuration module**

```rust
pub struct S3Connection { pub endpoint: String, pub access_key: String,
    pub secret_key: String, pub bucket: String, pub region: String }
pub struct Config { pub r2: S3Connection, pub rustfs: S3Connection }

impl Config {
    pub fn from_env() -> anyhow::Result<Self> { /* read the named environment variables */ }
    pub fn from_values(/* fields used in the tests */) -> anyhow::Result<Self> {
        /* construct R2 endpoint as https://{account}.r2.cloudflarestorage.com;
           default absent bucket names to aci; reject blank and placeholder destination endpoints */
    }
}
```

- [ ] **Step 4: Run the unit tests**

Run: `cargo test config`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add R2 and RustFS configuration"
```

### Task 2: Stream and resume object transfer

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs`

**Interfaces:**
- Consumes: `Config` and two configured `aws_sdk_s3::Client` values.
- Produces: `async fn copy_all(source: &Client, destination: &Client, config: &Config) -> anyhow::Result<TransferSummary>`.
- Produces: `TransferSummary { copied: u64, skipped: u64 }`.

- [ ] **Step 1: Add focused unit tests for small transfer decisions**

```rust
#[test]
fn skips_only_when_destination_size_matches_source() {
    assert!(should_skip(Some(42), Some(42)));
    assert!(!should_skip(Some(42), Some(41)));
    assert!(!should_skip(Some(42), None));
}
```

- [ ] **Step 2: Run the test to verify failure**

Run: `cargo test skips_only_when_destination_size_matches_source`

Expected: FAIL because `should_skip` does not exist.

- [ ] **Step 3: Implement paginated streaming copy**

```rust
let source_object = source.get_object().bucket(&config.r2.bucket).key(key).send().await?;
let mut put = destination.put_object()
    .bucket(&config.destination.bucket)
    .key(key)
    .body(source_object.body);
if let Some(content_type) = source_object.content_type() {
    put = put.content_type(content_type);
}
put.send().await?;
```

Use `head_object` against the destination before `get_object`; treat an S3 `NotFound` response as a copy requirement and return every other destination error. Continue using R2 `list_objects_v2` continuation tokens. Print the object key before each copy or skip and print copied/skipped totals at the end.

- [ ] **Step 4: Run all Rust checks**

Run: `cargo fmt --check && cargo test && cargo clippy -- -D warnings`

Expected: all commands exit 0.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: copy R2 objects to RustFS"
```

### Task 3: Operator configuration and preflight

**Files:**
- Create: `.env.example`
- Modify: `README.md`
- Test: manual command sequence in `README.md`

**Interfaces:**
- Consumes: environment variable names accepted by `Config::from_env`.
- Produces: an executable configuration and verification checklist.

- [ ] **Step 1: Add a non-secret environment template**

```dotenv
R2_ACCOUNT_ID=
R2_ACCESS_KEY=
R2_SECRET_KEY=
R2_BUCKET=aci
S3_ENDPOINT=s3-end-point-api-url
S3_ACCESS_KEY=
S3_SECRET_KEY=
S3_BUCKET=aci
```

- [ ] **Step 2: Document build, preflight, copy, and cutover**

Include these commands and requirements:

```bash
set -a; source .env; set +a
cargo run --release
```

State that the destination bucket `aci` must exist first, the placeholder endpoint must be replaced with a full `http://` or `https://` API URL, both credential pairs need S3 permissions, the copy should be rerun immediately before cutover, and Laravel should be switched only after count/sample verification. Note that RustFS and MinIO are supported through their S3-compatible APIs; local filesystem copying is not supported.

- [ ] **Step 3: Validate documentation and template**

Run: `cargo fmt --check && cargo test && git diff --check`

Expected: all commands exit 0.

- [ ] **Step 4: Commit**

```bash
git add .env.example README.md
git commit -m "docs: add RustFS migration instructions"
```
