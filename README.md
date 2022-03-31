
- use `./configure --without-isal` instead of `./configure` to avoid isa-unsupported or certain error when congifure SPDK
- add env variables explicitly 
    - `export CPATH=$PWD/spdk-sys/spdk/build/include`
    - `export LIBRARY_PATH=$PWD/spdk-sys/target:$PWD/spdk-sys/spdk/build/lib`
    - `export LD_LIBRARY_PATH=$PWD/spdk-sys/target:$PWD/spdk-sys/spdk/build/lib`
- run as rooter
`cargo run --example hello_blob ./examples/hello_blob.json`
