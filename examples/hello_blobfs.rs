//! This is an example for syncronous API of blobfs

use std::{
    sync::{Arc, Mutex},
};

use async_spdk::{
    event::{app_stop},
    thread::Poller,
    *,
};
use blobfs::*;
use log::*;

fn main() {
    env_logger::init();
    let fsflag = Arc::new(Mutex::new(false));
    let fs = Arc::new(Mutex::new(SpdkFilesystem::default()));
    let shutdown = Arc::new(Mutex::new(false));
    let shutdown_poller = Arc::new(Mutex::new(Poller::default()));

    let ff2 = fsflag.clone();
    let fs2 = fs.clone();
    let shutdown2 = shutdown.clone();

    let fs_handle = std::thread::spawn(|| {
        event::AppOpts::new()
            .name("hello_blobfs")
            .config_file(&std::env::args().nth(1).expect("no config file"))
            .reactor_mask("0x1")
            .block_on(async_main(ff2, fs2, shutdown2, shutdown_poller))
            .unwrap();
    });

    let ff3 = fsflag;
    let fs3 = fs;
    let shutdown3 = shutdown;

    test_fs(ff3, fs3, shutdown3);
    fs_handle.join().unwrap();
}

fn test_fs(
    fflag: Arc<Mutex<bool>>,
    fs: Arc<Mutex<SpdkFilesystem>>,
    shutdown: Arc<Mutex<bool>>,
) -> Result<()> {
    loop {
        if *fflag.lock().unwrap() {
            break;
        }
    }

    let fs = fs.lock().unwrap();
    info!("App thread get fs handle");

    if fs.is_null() {
        info!("fs pointer is null");
        return Ok(());
    }

    let ctx = fs.alloc_thread_ctx()?;
    info!("App thread alloc ctx");

    if fs.is_null() {
        info!("fs pointer is null");
        return Ok(());
    }

    fs.create(&ctx, "file1")?;
    info!("Create file1 success");

    if fs.is_null() {
        info!("fs pointer is null");
        return Ok(());
    }

    fs.delete(&ctx, "file1")?;
    info!("Delete file1 success");

    *shutdown.lock().unwrap() = true;
    info!("set shutdown to true");

    Ok(())
}

async fn async_main(
    fflag: Arc<Mutex<bool>>,
    fs: Arc<Mutex<SpdkFilesystem>>,
    shutdown: Arc<Mutex<bool>>,
    shutdown_poller: Arc<Mutex<Poller>>,
) -> Result<()> {
    info!("start main: hello_blobfs");

    let mut bdev = blob_bdev::BlobStoreBDev::create("Malloc0")?;
    info!("BlobStoreBdev created");

    let mut blobfs_opt = SpdkBlobfsOpts::init().await?;
    info!("BlobFs opts init");

    let blobfs = SpdkFilesystem::init(&mut bdev, &mut blobfs_opt).await?;
    info!("BlobFs init");

    let shutdown_fs = fs.clone();
    let shutdown_copy = shutdown.clone();
    let shutdown_poller_copy = shutdown_poller.clone();

    *shutdown_poller.lock().unwrap() = Poller::register(move || {
        if *shutdown_copy.lock().unwrap() {
            info!("shutdonw poller receive shutdown signal");
            shutdown_fs.lock().unwrap().unload_sync();
            shutdown_poller_copy.lock().unwrap().unregister();
            app_stop();
        }
        true
    })?;

    *fs.lock().unwrap() = blobfs;
    info!("Pass fs to global");

    *fflag.lock().unwrap() = true;
    info!("Set flag to true");

    Ok(())
}
