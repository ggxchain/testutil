These metadata files are generated with `fetch_metadata.sh` script. If you update GGX version in `ggs.rs`, please run
this script to update metadata files.

We intentionally do not create mod.rs file here to avoid unnecessary `subxt` calls which generates and compiles
metadata (that takes ~4m).
