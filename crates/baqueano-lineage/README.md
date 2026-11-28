# baqueano-lineage

Read [Nextflow](https://www.nextflow.io) data lineage stores from Rust.

> **Status: pre-alpha.** The API will change before 1.0.

Nextflow (25.04 and later) records every workflow run, task and output file as a
`lineage/v1beta1` JSON record in a `.lineage` directory. `baqueano-lineage` reads those stores without
a JVM and behaves like `nextflow lineage`:

- **Parse** lineage IDs (`Lid`) and records (`Record`) written by Nextflow 25.04 through 26.10.
  Every field is optional and unknown fields are kept, so newer records still parse.
- **Read** a store on local disk, S3, Google Cloud Storage or Azure (`ObjectLineageStore`, built
  on [`object_store`](https://docs.rs/object_store)): list runs, get records, `find` by property.
- **Check** a data file against its recorded checksum (`check`), local paths and `s3://`.
- **Diff** two runs or records semantically (`diff`) or line by line like the CLI (`diff_raw`).
- **View** a record or one of its properties (`view`), and walk a file's **provenance** back to
  the pipeline inputs (`provenance`).

It is read-only: it never writes, deletes or creates objects.

## Example

```rust,no_run
use baqueano_lineage::{CheckOptions, FindQuery, LineageStore, ObjectLineageStore, check, diff};

#[tokio::main]
async fn main() -> Result<(), baqueano_lineage::LineageError> {
    // A local `.lineage` directory; `s3://bucket/prefix`, `gs://…` and `az://…` work too.
    let store = ObjectLineageStore::open(".lineage")?;

    // Runs in launch order, like `nextflow lineage list`.
    let runs = store.list_runs().await?;
    for run in &runs {
        println!("{} {} {}", run.timestamp, run.run_name, run.lid);
    }

    // Verify every output file, like `nextflow lineage check`.
    let files = store.find(&FindQuery::parse_args(["type=FileOutput"])?).await?;
    for lid in &files {
        let report = check(&store, lid, &CheckOptions::default()).await?;
        println!("{lid} {:?}", report.status);
    }

    // What changed between the last two runs (params, scripts, tasks, outputs)?
    if let [.., a, b] = runs.as_slice() {
        for change in diff(&store, &a.lid, &b.lid).await?.changes {
            println!("{}: {:?} -> {:?}", change.path, change.a, change.b);
        }
    }
    Ok(())
}
```

Cloud settings come from the environment (`AWS_*`, `GOOGLE_*`, `AZURE_*`), or are passed
explicitly as `object_store` config keys:

```rust,no_run
# fn main() -> Result<(), baqueano_lineage::LineageError> {
use baqueano_lineage::ObjectLineageStore;

let store = ObjectLineageStore::open_with_options(
    "s3://my-bucket/.lineage",
    [("aws_region", "eu-west-1")],
)?;
# Ok(())
# }
```

## Features

`aws`, `gcp` and `azure` (all on by default) enable `s3://`, `gs://` and `az://` stores. Local
paths and `file://` always work.

## License

Apache-2.0
