//! This is an example for syncronous API of blobfs

use async_spdk::*;
use blobfs::*;
use log::*;

fn main() {
    env_logger::init();
    event::AppOpts::new()
        .name("hello_blobfs")
        .config_file(&std::env::args().nth(1).expect("no config file"))
        .block_on(async_main())
        .unwrap();
}

async fn async_main() -> Result<()> {
    info!("start main: hello_blobfs");

    let mut bdev = blob_bdev::BlobStoreBDev::create("Malloc0")?;
    info!("BlobStoreBdev created");

    let mut blobfs_opt = SpdkBlobfsOpts::init().await?;
    info!("BlobFs opts init");

    let blobfs = SpdkFilesystem::init(&mut bdev, &mut blobfs_opt).await?;
    info!("BlobFs init");

    let mut ctx = blobfs.alloc_thread_ctx()?;
    info!("BlobFsThreadCtx allocated");

    blobfs.create(&ctx, "file1")?;
    info!("create file1 succeed");

    blobfs.create(&ctx, "file2")?;
    info!("create file2 succeed");

    let mut file1 = SpdkFile::default();
    blobfs.open(&ctx, "file1", 1u32, &mut file1)?;
    info!("file1 open succeed");

    let write_buf = b"hello";
    file1
        .write(&ctx, write_buf, 0, write_buf.len() as u64)
        .unwrap();
    info!("file1 write succeed");

    let mut read_buf = [0u8; 5];
    let size = file1.read(&ctx, &mut read_buf, 0, write_buf.len() as u64)?;
    info!("file1 read succeed");

    for i in 0..write_buf.len() {
        if read_buf[i] != write_buf[i] {
            error!(
                "Data mismatch on {}, read: {}, write: {}",
                i, read_buf[i], write_buf[i]
            );
        }
    }
    info!("data match!");

    let mut stat = SpdkFileStat::default();
    blobfs.stat(&ctx, "file1", &mut stat)?;
    info!("file1 stat: {:?}", stat);

    blobfs.rename(&ctx, "file2", "file3")?;
    info!("rename succeed");

    blobfs.delete(&ctx, "file1")?;
    blobfs.delete(&ctx, "file3")?;
    info!("delete succeed");

    blobfs.unload().await?;
    info!("blobfs unload succeed");

    Ok(())
}
