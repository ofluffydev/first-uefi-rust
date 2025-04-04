#![no_main]
#![no_std]

use log::info;
use uefi::{boot::open_protocol_exclusive, prelude::*, proto::console::text::Input};


#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    info!("Hello world!");

    let handle = uefi::boot::get_handle_for_protocol::<Input>().unwrap();
    let input = open_protocol_exclusive::<Input>(handle).unwrap();
    let event = input.get().unwrap().wait_for_key_event().unwrap();
    boot::wait_for_event(&mut [event]).expect("Failed to wait for event");
    info!("Key pressed! Leaving now...");
    boot::stall(5_000_000);
    
    Status::SUCCESS
}
