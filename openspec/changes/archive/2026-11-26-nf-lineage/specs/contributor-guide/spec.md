# Delta for contributor-guide

## ADDED Requirements

### Requirement: Integration test instructions

`CONTRIBUTING.md` MUST explain how to run the S3 integration tests locally: start
`ministackorg/ministack:1.5.45` on port 4566 with Docker, set `NF_LINEAGE_MINISTACK_URL`, run
`cargo test -p nf-lineage --test s3_ministack`, and state that without the variable the tests skip.

#### Scenario: Newcomer runs S3 tests (V-doc)
- GIVEN `CONTRIBUTING.md`
- WHEN grepped for `ministackorg/ministack:1.5.45`, `NF_LINEAGE_MINISTACK_URL`, `s3_ministack`
- THEN all three match
