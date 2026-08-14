# R2 to S3-compatible storage migration design

## Goal

Copy every object in the Cloudflare R2 bucket `aci` into an S3-compatible destination bucket `aci` (such as RustFS or MinIO) without modifying or deleting source objects.

## Configuration

The migrator receives all connection details from environment variables:

- `R2_ACCOUNT_ID`, `R2_ACCESS_KEY`, `R2_SECRET_KEY`, and `R2_BUCKET` configure the R2 source.
- `S3_ENDPOINT` configures the destination S3-compatible API endpoint. Its initial value is the placeholder `s3-end-point-api-url` and must be replaced before execution.
- `S3_ACCESS_KEY`, `S3_SECRET_KEY`, and `S3_BUCKET` configure the destination. The destination bucket defaults to `aci`.

## Data flow

The program lists R2 objects using continuation tokens. For each object, it downloads from R2 and uploads to RustFS using the same object key. It carries the original content type when R2 supplies one. No object is deleted from R2.

Before copying, it checks RustFS for an object at the same key. If it exists with the same content length, the object is skipped. This makes a failed run resumable. A matching length is a resume optimisation, not a cryptographic integrity guarantee.

## Errors and verification

The program stops with a non-zero exit status on an API or transfer failure and identifies the object being processed. It reports copied and skipped object counts on completion. Operators should compare the R2 and destination object counts after the copy, run the program once more immediately before the Laravel cutover, then change Laravel's S3 endpoint and credentials only after verification.

## Scope

The migrator copies object data and content type. Bucket-policy, lifecycle, CORS, custom-domain, and metadata migration are outside this utility's scope and must be configured in the destination separately. Local filesystem copying is not supported.
