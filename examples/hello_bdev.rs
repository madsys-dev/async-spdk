use async_spdk::*;
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
    info!("start main: hello_bdev");

    let bdev_desc = BDevDesc::create_desc("Malloc0")?;
    info!("get bdev descriptor");

    let Bdev = bdev_desc.get_bdev()?;
    info!("get bdev pointer by descriptor");

    let blk_size = Bdev.get_block_size();
    info!("get block size: {}", blk_size);

    let balign = Bdev.get_buf_align();
    info!("get buffer align");

    let mut write_buf = dma_buf::new(blk_size as u64, balign as u64)?;
    write_buf.fill(0x5a);

    let channel = bdev_desc.get_io_channel()?;

    info!("start writing");
    bdev_desc
        .write(&channel, 0, write_buf.len() as u64, write_buf.as_slice())
        .await?;
    info!("finish writing");

    let mut read_buf = dma_buf::new(blk_size as u64, balign as u64)?;

    info!("start reading");
    bdev_desc
        .read(&channel, 0, read_buf.len() as u64, read_buf.as_mut_slice())
        .await?;
    info!("finish reading");

    if write_buf.as_slice() != read_buf.as_slice() {
        error!("inconsistent data!");
    } else {
        info!("data matches!");
    }

    Bdev.release_io_channel(channel);
    info!("channel released");

    bdev_desc.close();
    info!("bdev closed");

    drop(write_buf);
    drop(read_buf);

    Ok(())
}
