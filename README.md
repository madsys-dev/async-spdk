
- use ./configure --without-isal instead of ./configure to avoid isa-unsupported or certain error when congifure SPDK
- run as rooter
    - cargo run --example hello_blob ./examples/hello_blob.json
    - cargo run --example hello_bdev ./examples/hello_bdev.json
- when miss hugepage
    - echo "1024" > /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages
