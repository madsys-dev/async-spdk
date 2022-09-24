use async_spdk::{event::app_stop, *};
use bdev::*;
use log::*;

fn main() {
    env_logger::init();
    event::AppOpts::new()
        .name("hello_bdev")
        .config_file(&std::env::args().nth(1).expect("no config_file"))
        .block_on(async_main())
        .unwrap();
}

async fn async_main() -> Result<()> {
    info!("Start main: hello_bdev");

    let bdev_desc = BdevDesc::create_desc("Malloc0")?;
    info!("Get bdev descriptor");

    let Bdev = bdev_desc.get_bdev()?;
    info!("Get bdev pointer by descriptor");

    let blk_size = Bdev.get_block_size();
    info!("Get block size: {}", blk_size);

    let balign = Bdev.get_buf_align();
    info!("Get buffer align: {}", balign);

    let mut write_buf = env::DmaBuf::alloc(blk_size as usize, 0x1000);
    write_buf.as_mut().fill(0x5a);

    let channel = bdev_desc.get_io_channel()?;
    info!("IO channel get");

    bdev_desc
        .write(&channel, 0, blk_size as u64, write_buf.as_ref())
        .await?;

    info!("Finish writing");

    let mut read_buf = env::DmaBuf::alloc(blk_size as usize, 0x1000);

    bdev_desc
        .read(&channel, 0, blk_size as u64, read_buf.as_mut())
        .await?;
    info!("Finish reading");

    if write_buf.as_ref() != read_buf.as_ref() {
        error!("Inconsistent data!");
    } else {
        info!("Data matches!");
    }

    bdev_desc.close();
    info!("Bdev closed");

    // attention! io channel and dma buffer is dropped automatically
    // since we implement drop trait
    // don't need to call any free API
    // TODO: any other struct need to implement DROP for more convenience?

    app_stop();

    Ok(())
}
