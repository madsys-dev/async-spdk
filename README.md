
- use `./configure --without-isal` instead of `./configure` to avoid isa-unsupported or certain error
- add env variables explicitly 
    - `export CPATH=$PWD/spdk-sys/spdk/build/include`
    - `export LIBRARY_PATH=$PWD/spdk-sys/target:$PWD/spdk-sys/spdk/build/lib`
- run with root
`cargo run --example hello_blob hello_blob.json`