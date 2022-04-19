use log::*;
use storage_engine::*;
use async_spdk::*;

fn main(){
    env_logger::init();
    event::AppOpts::new()
        .name("madio_test")
        .config_file(&std::env::args().nth(1).expect("expect config file"))
        .block_on(test_helper("madio"))
        .unwrap();
}

async fn test_helper(name: &str)->Result<()>{
    let de = DeviceEngine::new(name).await?;
    let b = de.create_blob(4).await?;
    let bid = b.get_id()?;
    let io_unit_size = de.get_io_size()?;

    // prepare read & write buffer
    // TODO: this should be reorganized
    let mut write_buf = env::DmaBuf::alloc(io_unit_size as usize, 0x1000);
    write_buf.as_mut().fill(0x5a);
    let mut read_buf = env::DmaBuf::alloc(io_unit_size as usize, 0x1000);
    de.write(0, bid, write_buf.as_ref()).await?;
    de.read(0, bid, read_buf.as_mut()).await?;

    // check whether data matches
    if write_buf.as_ref() != read_buf.as_ref(){
        error!("fail test!");
    }else{
        info!("pass test...");
    }

    de.delete_blob(bid).await?;
    de.close_bs().await?;
    Ok(())
}
