use async_spdk::*;
use log::*;

fn main() {
    env_logger::init();
    event::AppOpts::new()
        .name("hello_blob")
        .config_file(&std::env::args().nth(1).expect("no config_file"))
        .block_on(async_main())
        .unwrap();
}

async fn async_main() -> Result<()> {
    info!("start main");

    info!("testing future spawn");
    let ret = event::spawn(async {
        info!("new future is running");
        1
    })
    .await;
    assert_eq!(ret, 1);
    info!("future joined");

    let mut bs_dev = blob_bdev::BlobStoreBDev::create("Malloc0")?;
    info!("BlobstoreBdev created");

    let blobstore = blob::Blobstore::init(&mut bs_dev).await?;
    info!("Blobstore created");

    let io_unit_size = blobstore.io_unit_size();
    info!("IO unit size: {:?}", io_unit_size);

    let blob_id = blobstore.create_blob().await?;
    info!("Blob created: {:?}", blob_id);

    let blob = blobstore.open_blob(blob_id).await?;
    info!("Opened blob");

    let free_clusters = blobstore.free_cluster_count();
    info!("blobstore has FREE clusters of {:?}", free_clusters);

    blob.resize(free_clusters).await?;

    let total = blob.num_clusters();
    info!("resized blob now has USED clusters of {}", total);

    blob.sync_metadata().await?;
    info!("metadata sync complete");

    /*
     * Buffers for data transfer need to be allocated via SPDK. We will
     * tranfer 1 page of 4K aligned data at offset 0 in the blob.
     */
    let mut write_buf = env::DmaBuf::alloc(io_unit_size as usize, 0x1000);
    write_buf.as_mut().fill(0x5a);

    /* Now we have to allocate a channel. */
    let channel = blobstore.alloc_io_channel()?;

    /* Let's perform the write, 1 page at offset 0. */
    info!("Starting write");
    blob.write(&channel, 0, write_buf.as_ref()).await?;
    info!("Finished writing");

    let mut read_buf = env::DmaBuf::alloc(io_unit_size as usize, 0x1000);

    /* Issue the read */
    info!("Starting read");
    blob.read(&channel, 0, read_buf.as_mut()).await?;
    info!("Finished read");

    /* Now let's make sure things match. */
    if write_buf.as_ref() != read_buf.as_ref() {
        error!("Error in data compare");
    } else {
        info!("read SUCCESS and data matches!");
    }

    /* Now let's close it and delete the blob in the callback. */
    blob.close().await?;
    info!("Closed");

    blobstore.delete_blob(blob_id).await?;
    info!("Deleted");

    // XXX: io_channel must be dropped before unload.
    // TODO: find a way to force that in Rust
    // drop(channel);
    blobstore.unload().await?;
    info!("Blobstore unloaded");

    Ok(())
}
