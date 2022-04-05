use async_spdk::*;
use log::*;
use bdev::BDevDesc;

fn main(){
    env_logger::init();
    event::AppOpts::new()
        .name("hello_bdev")
        .config_file(&std::env::args().nth(1).expect("no config file"))
        .block_on(async_main())
        .unwrap();
}

async fn async_main() -> Result<()>{
    info!("start main: hello_bdev");

    let bdev_desc = BDevDesc::create_desc("Malloc0")?;
    info!("get bdev descriptor");

    let Bdev = bdev_desc.get_bdev()?;
    info!("get bdev pointer by descriptor");

    let blk_size = Bdev.get_block_size();
    info!("get block size");

    let balign = Bdev.get_buf_align();
    info!("get buffer align");

    let mut write_buf = env::DmaBuf::alloc(blk_size as usize, balign);
    write_buf.as_mut().fill(0x5a);

    let channel = bdev_desc.get_io_channel()?;

    info!("start writing");
    bdev_desc.write(&channel, 0, write_buf.as_ref()).await?;
    info!("finish writing");

    let mut read_buf = env::DmaBuf::alloc(blk_size as usize, balign);

    info!("start reading");
    bdev_desc.read(&channel, 0, read_buf.as_mut()).await?;
    info!("finish reading");

    if write_buf.as_ref() != read_buf.as_ref(){
        error!("inconsistent data!");
    }else{
        info!("data matches!");
    }

    Bdev.release_io_channel(channel);
    info!("channel released");

    bdev_desc.close();
    info!("bdev closed");

    Ok(())
}

