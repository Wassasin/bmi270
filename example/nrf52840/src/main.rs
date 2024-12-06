#![no_std]
#![no_main]

use core::i16;

use bmi270::{config::BMI270_CONFIG_FILE, ll};
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts, peripherals,
    twim::{self, Frequency},
};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    TWISPI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("running!");

    let scl = p.P1_15;
    let sda = p.P1_14;

    let mut config = twim::Config::default();
    config.frequency = Frequency::K400;
    config.sda_pullup = true;
    config.scl_pullup = true;
    let twim = twim::Twim::new(p.TWISPI0, Irqs, sda, scl, config);

    let mut bmi = ll::Device::new(ll::DeviceInterface::new(twim));
    let chip_id = bmi.chip_id().read_async().await.unwrap();
    defmt::info!("{:?}", chip_id);

    bmi.cmd()
        .write_async(|w| w.set_cmd(ll::Command::Softreset))
        .await
        .unwrap();

    Timer::after_millis(10).await;

    bmi.pwr_conf()
        .modify_async(|w| w.set_adv_power_save(false))
        .await
        .unwrap();

    Timer::after_millis(1).await;

    bmi.init_ctrl()
        .write_async(|w| w.set_init_ctrl(0x00))
        .await
        .unwrap();

    const CHUNK_SIZE: usize = 512;
    let mut addr = 0;
    for chunk in BMI270_CONFIG_FILE.chunks(CHUNK_SIZE) {
        bmi.init_addr()
            .write_async(|w| {
                let addr = (addr / 2) as u16;
                w.set_init_addr_0((addr & 0x0F) as u8);
                w.set_init_addr_1((addr >> 4) as u8);
            })
            .await
            .unwrap();

        bmi.init_data().write_async(chunk).await.unwrap();

        addr += chunk.len();
    }

    bmi.init_ctrl()
        .write_async(|w| w.set_init_ctrl(0x01))
        .await
        .unwrap();

    bmi.pwr_ctrl()
        .modify_async(|w| {
            w.set_acc_en(true);
            w.set_gyr_en(false);
        })
        .await
        .unwrap();

    bmi.acc_conf()
        .modify_async(|w| {
            w.set_acc_odr(ll::AccOdr::Odr1P5);
            w.set_acc_bwp(ll::AccBwp::NormAvg4); // Osr4Avg1
            w.set_acc_filter_perf(ll::AccFilterPerf::Ulp)
        })
        .await
        .unwrap();

    bmi.read_all_registers_async(|i, name, field_set_value| {
        defmt::info!("{} {} {}", i, name, field_set_value);
    })
    .await
    .unwrap();

    loop {
        let status = bmi.status().read_async().await.unwrap();
        if !status.drdy_acc() {
            Timer::after_millis(1).await;
            continue;
        }

        let acc = bmi.acc().read_async().await.unwrap();
        let x = acc.x() as f32 / 16384. * 4.;
        let y = acc.y() as f32 / 16384. * 4.;
        let z = acc.z() as f32 / 16384. * 4.;
        defmt::info!("{} {} {}", x, y, z);
    }

    // veml.command_code()
    //     .write_async(|w| {
    //         w.set_sd_0(false);
    //         w.set_sd_1(false);
    //         w.set_sd_als(false);
    //         w.set_af(ll::Af::AutoMode);
    //         w.set_trig(false);
    //         w.set_it(ll::It::It50Ms);
    //     })
    //     .await
    //     .unwrap();

    // loop {
    //     veml.read_all_registers_async(|i, name, field_set_value| {
    //         defmt::info!("{} {} {}", i, name, field_set_value);
    //     })
    //     .await
    //     .unwrap();
    //     Timer::after(Duration::from_millis(500)).await;
    // }
}
