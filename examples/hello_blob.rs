use async_spdk::*;
use log::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let mut bs_dev = blob_bdev::BlobStoreBDev::create("Malloc0")?;

    let blobstore = blob::Blobstore::init(&mut bs_dev).await?;

    let page_size = blobstore.page_size();
    info!("Page size: {:?}", page_size);

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
    let mut write_buf = env::DmaBuf::alloc(page_size as usize, 0x1000);
    write_buf.as_mut().fill(0x5a);

    /* Now we have to allocate a channel. */
    let channel = blobstore.alloc_io_channel()?;

    /* Let's perform the write, 1 page at offset 0. */
    info!("Starting write");
    blob.write(&channel, 0, write_buf.as_ref()).await?;
    info!("Finished writing");

    let mut read_buf = env::DmaBuf::alloc(page_size as usize, 0x1000);

    /* Issue the read */
    info!("Starting read");
    blob.read(&channel, 0, read_buf.as_mut()).await?;
    info!("Finished read");

    /* Now let's make sure things match. */
    if write_buf.as_ref() != read_buf.as_ref() {
        info!("Error in data compare");
    } else {
        info!("read SUCCESS and data matches!");
    }

    /* Now let's close it and delete the blob in the callback. */
    blob.close().await?;
    info!("Closed");

    blobstore.delete_blob(blob_id).await?;
    info!("Deleted");

    blobstore.unload().await?;
    info!("Blobstore unloaded");

    Ok(())
}
