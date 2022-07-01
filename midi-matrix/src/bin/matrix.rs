#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(generic_const_exprs)]

use core::convert::Infallible;

use defmt::*;
use defmt_rtt as _;
use eg_bdf::{include_bdf, text::BdfTextStyle, BdfFont};
use embassy::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Sender},
    executor::Spawner,
    time::{with_timeout, Duration, Timer},
    util::{select4, Either4, Forever, Select},
};
use embassy_stm32::{
    dma::NoDma,
    exti::ExtiInput,
    gpio::{Input, Level, Output, Pin, Pull, Speed},
    peripherals::PC14,
    spi::{self, Spi},
    time::U32Ext,
    usb_otg::{State, Usb, UsbBus, UsbOtg, UsbSerial},
    Config, Peripherals,
};
use panic_probe as _;

static CHANNEL: Forever<Channel<NoopRawMutex, usize, 1>> = Forever::new();

#[embassy::main]
async fn main(spawner: Spawner, p: Peripherals) {
    warn!("use open drain outputs!");
    let mut buttons = (
        ExtiInput::new(Input::new(p.PA4, Pull::Up), p.EXTI4),
        ExtiInput::new(Input::new(p.PA5, Pull::Up), p.EXTI5),
        ExtiInput::new(Input::new(p.PA6, Pull::Up), p.EXTI6),
        ExtiInput::new(Input::new(p.PA7, Pull::Up), p.EXTI7),
    );

    let driv0r = Output::new(p.PA9.degrade(), Level::High, Speed::Medium);
    let mut drivers = [
        &mut Output::new(p.PA0.degrade(), Level::High, Speed::Medium),
        &mut Output::new(p.PA1.degrade(), Level::High, Speed::Medium),
        &mut Output::new(p.PA2.degrade(), Level::High, Speed::Medium),
        &mut Output::new(p.PA3.degrade(), Level::High, Speed::Medium),
    ];
    loop {
        for driver in drivers.iter_mut() {
            driver.set_low();
        }

        let f1 = buttons.0.wait_for_falling_edge();
        let f2 = buttons.1.wait_for_falling_edge();
        let f3 = buttons.2.wait_for_falling_edge();
        let f4 = buttons.3.wait_for_falling_edge();

        let sel_fut = select4(f1, f2, f3, f4);

        sel_fut.await;

        for driver in drivers.iter_mut() {
            driver.set_high();
        }
        Timer::after(Duration::from_millis(5)).await;

        for (row, driver) in drivers.iter_mut().enumerate() {
            driver.set_low();

            let bs = [
                buttons.0.is_low(),
                buttons.1.is_low(),
                buttons.2.is_low(),
                buttons.3.is_low(),
            ];

            for (col, state) in bs.iter().enumerate() {
                if *state {
                    info!("{}", 4 * row + col + 1);
                }
            }

            driver.set_high();
        }
    }
}

#[embassy::task]
async fn send_task(
    mut butt: ExtiInput<'static, PC14>,
    sender: Sender<'static, NoopRawMutex, usize, 1>,
) {
    let mut idx = 0;
    sender.send(idx).await;
    loop {
        butt.wait_for_falling_edge().await;
        Timer::after(Duration::from_millis(10)).await;
        if butt.is_low() {
            idx = (idx + 1);
            sender.send(idx).await;
        }
    }
}
