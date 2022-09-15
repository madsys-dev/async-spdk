//! This is an example for syncronous API of blobfs

use std::{sync::{
    // atomic::AtomicBool, 
    Arc, Mutex}, cell::RefCell};
// use std::cell::RefCell;
// use lazy_static::lazy_static;

use async_spdk::{
*, event::{send_shutdown, app_fini}, thread::Poller};
use blobfs::*;
use log::*;

// static mut FSFLAG: AtomicBool = AtomicBool::new(false);
// lazy_static!{
//     static ref FS: Arc<Mutex<Option<SpdkFilesystem>>> = Arc::new(Mutex::new(None));
// }
// static SHUTDOWN: bool = false;

fn main() {
    env_logger::init();
    let mut fsflag = Arc::new(Mutex::new(false));
    let mut fs = Arc::new(Mutex::new(SpdkFilesystem::default()));
    let mut shutdown = Arc::new(Mutex::new(false));

    let ff2 = fsflag.clone();
    let fs2 = fs.clone();
    let shutdown2 = shutdown.clone();

    let fs_handle = std::thread::spawn(|| {
        event::AppOpts::new()
            .name("hello_blobfs")
            .config_file(&std::env::args().nth(1).expect("no config file"))
            .reactor_mask("0x3")
            .block_on(async_main(ff2, fs2, shutdown2))
            .unwrap();
        app_fini();
    });

    let ff3 = fsflag.clone();
    let fs3 = fs.clone();
    let shutdown3 = shutdown.clone();

    test_fs(ff3, fs3, shutdown3);
    fs_handle.join().unwrap();
}

fn test_fs(
    fflag: Arc<Mutex<bool>>,
    fs: Arc<Mutex<SpdkFilesystem>>,
    shutdown: Arc<Mutex<bool>>,
) -> Result<()> {
    loop {
        if *fflag.lock().unwrap() == true{
            break;
        }
    }

    let fs = fs.lock().unwrap();
    info!("App thread get fs handle");

    if fs.is_null(){
        info!("fs pointer is null");
        return Ok(())
    }

    let ctx = fs.alloc_thread_ctx()?;
    info!("App thread alloc ctx");

    if fs.is_null(){
        info!("fs pointer is null");
        return Ok(())
    }

    info!("start create");
    fs.create(&ctx, "file1")?;
    info!("Create file1 success");

    if fs.is_null(){
        info!("fs pointer is null");
        return Ok(())
    }

    fs.delete(&ctx, "file1")?;
    info!("Delete file1 success");

    // send_shutdown();

    *shutdown.lock().unwrap() = true;
    info!("set shutdown to true");

    // let fs = FS.unwrap().lock().unwrap();
    // unsafe {
    //     let fs = FS.lock().unwrap();
    //     info!("App thread get fs handle");

    //     let ctx = fs.unwrap().alloc_thread_ctx()?;
    //     info!("App thread alloc ctx");

    //     fs.unwrap().create(&ctx, "file1")?;
    //     info!("Create file1 success");

    //     fs.unwrap().delete(&ctx, "file1")?;
    //     info!("Delete file1 success");
    // };
    // let fs = unsafe {let t = FS.lock().unwrap().as_ref();
    // t};
    // info!("App thread get fs handle");

    // let mut ctx = fs.unwrap().alloc_thread_ctx()?;
    // info!("App thread alloc ctx");

    // fs.unwrap().create(&ctx, "file1")?;
    // info!("Create file1 success");

    // fs.unwrap().delete(&ctx, "file1")?;
    // info!("Delete file1 success");

    // drop(fs);
    // info!("App thread drop fs");

    Ok(())
}

async fn async_main(
    fflag: Arc<Mutex<bool>>,
    fs: Arc<Mutex<SpdkFilesystem>>,
    shutdown: Arc<Mutex<bool>>,
) -> Result<()> {
    info!("start main: hello_blobfs");

    if *fflag.lock().unwrap() == true{
        info!("flag is ok");
        return Ok(());
    }

    let mut bdev = blob_bdev::BlobStoreBDev::create("Malloc0")?;
    info!("BlobStoreBdev created");

    let mut blobfs_opt = SpdkBlobfsOpts::init().await?;
    info!("BlobFs opts init");

    let blobfs = SpdkFilesystem::init(&mut bdev, &mut blobfs_opt).await?;
    info!("BlobFs init");

    let shutdown_fs = fs.clone();
    let shutdown_copy = shutdown.clone();

    let mut shutdown_poller = Poller::register(move ||{
        info!("shutdown poller is called");
        if *shutdown_copy.lock().unwrap() == true{
            info!("shutdonw poller receive shutdown signal");
            shutdown_fs.lock().unwrap().unload();
            info!("blobfs unload success");
        }
        true
    })?;

    *fs.lock().unwrap() = blobfs;
    info!("Pass fs to global");

    *fflag.lock().unwrap() = true;
    info!("Set flag to true");

    Ok(())
}
