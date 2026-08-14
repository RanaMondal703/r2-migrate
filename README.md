# R2 to S3-compatible storage migrator

Copies every object from a Cloudflare R2 bucket to an S3-compatible destination, such as RustFS or MinIO. The source bucket is read-only: this program never deletes or changes R2 objects. It does not support local filesystem destinations.

## Configure

1. Create the destination bucket (normally `aci`) in RustFS or MinIO.
2. Copy `.env.example` to a private `.env` file and add both the R2 and destination S3 credentials. Do not commit `.env`.
3. Replace `S3_ENDPOINT=s3-end-point-api-url` with the full destination S3 API URL, including `http://` or `https://`. For example: `https://minio.example.com`.
4. Ensure the R2 key can list and read `aci`, and the destination key can read object metadata and write objects to its target bucket.

`R2_BUCKET` and `S3_BUCKET` default to `aci` when empty. The migrator uses path-style S3 requests, which work well with MinIO and most self-hosted S3-compatible services.

To test locally without touching real R2, run the source against a local MinIO container and set `R2_ENDPOINT` to its URL (e.g. `http://127.0.0.1:9000`); leave it empty in production to use the real `https://<R2_ACCOUNT_ID>.r2.cloudflarestorage.com` endpoint.

## Run

```bash
set -a
source .env
set +a
cargo run --release
```

The program skips a destination object when it already has the same byte size, so it is safe to rerun after an interrupted copy. A matching size is a resume optimization, not a checksum verification.

## Cutover checklist

1. Run the copy while Laravel is still writing to R2.
2. Compare object counts and sample files from R2 and the destination.
3. Pause uploads briefly, run the copy again, and verify the final delta.
4. Update Laravel's S3 endpoint, credentials, bucket, and public URL/CDN configuration.
5. Keep R2 untouched until the application has operated successfully from the new storage.
