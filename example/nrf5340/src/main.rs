#![no_std]
#![no_main]

use bmi270::{
    config::BMI270_CONFIG_FILE,
    ll::{self, StatusMessage},
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Level, Output, Pull},
    peripherals,
    twim::{self, Frequency},
};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    SERIAL0 => twim::InterruptHandler<peripherals::SERIAL0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("running!");

    let scl = p.P1_09;
    let sda = p.P1_08;
    let int1 = p.P1_10;
    let int2 = p.P1_11;

    let led = p.P0_28;
    let mut led = Output::new(
        led,
        Level::High,
        embassy_nrf::gpio::OutputDrive::Standard0Disconnect1,
    );

    let mut int1 = Input::new(int1, Pull::Up);
    let mut int2 = Input::new(int2, Pull::Up);

    let mut config = twim::Config::default();
    config.frequency = Frequency::K400;
    config.sda_pullup = true;
    config.scl_pullup = true;
    let twim = twim::Twim::new(p.SERIAL0, Irqs, sda, scl, config);

    let mut bmi = ll::Device::new(ll::DeviceInterface::new(twim, ll::Address::Default));
    let chip_id = bmi.chip_id().read_async().await.unwrap();
    assert_eq!(chip_id.chip_id(), 0x24);

    bmi.cmd()
        .write_async(|w| w.set_cmd(ll::Command::Softreset))
        .await
        .unwrap();

    Timer::after_millis(20).await;

    bmi.pwr_conf()
        .modify_async(|w| w.set_adv_power_save(false))
        .await
        .unwrap();

    Timer::after_micros(450).await;

    bmi.init_ctrl()
        .write_async(|w| w.set_init_ctrl(0x00))
        .await
        .unwrap();

    const CHUNK_SIZE: usize = 16;
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

    loop {
        let internal_status = bmi.internal_status().read_async().await.unwrap();
        if internal_status.message() == StatusMessage::InitOk {
            break;
        }
        Timer::after_millis(1).await;
    }

    bmi.pwr_ctrl()
        .modify_async(|w| {
            w.set_acc_en(true);
            w.set_gyr_en(false);
            w.set_temp_en(false);
        })
        .await
        .unwrap();

    bmi.acc_conf()
        .modify_async(|w| {
            w.set_acc_odr(ll::AccOdr::Odr1P5);
            w.set_acc_bwp(ll::AccBwp::Osr4Avg1); // Osr4Avg1
            w.set_acc_filter_perf(ll::AccFilterPerf::Ulp)
        })
        .await
        .unwrap();

    bmi.int_latch()
        .write_async(|w| {
            w.set_int_latch(ll::LatchMode::Permanent);
        })
        .await
        .unwrap();

    bmi.int_1_io_ctrl()
        .write_async(|w| {
            w.set_output_en(true);
            w.set_od(ll::IntPinOd::OpenDrain);
            w.set_lvl(ll::IntPinLevel::ActiveLow);
        })
        .await
        .unwrap();

    bmi.int_2_io_ctrl()
        .write_async(|w| {
            w.set_output_en(true);
            w.set_od(ll::IntPinOd::OpenDrain);
            w.set_lvl(ll::IntPinLevel::ActiveLow);
        })
        .await
        .unwrap();

    bmi.int_map_data()
        .write_async(|w| {
            w.set_drdy_int_1(true);
        })
        .await
        .unwrap();

    // Reset registers to default and enable.
    bmi.feat_page()
        .write_async(|w| w.set_page(0x1))
        .await
        .unwrap();
    bmi.any_motion_1()
        .write_async(|w| {
            w.set_duration(0x2);
        })
        .await
        .unwrap();
    bmi.any_motion_2()
        .write_async(|w| {
            w.set_enable(true);
        })
        .await
        .unwrap();

    bmi.int_2_map_feat()
        .write_async(|w| {
            w.set_any_motion_out(true);
        })
        .await
        .unwrap();

    bmi.fifo_config_0()
        .write_async(|w| w.set_fifo_time_en(false))
        .await
        .unwrap();
    bmi.fifo_config_1()
        .write_async(|w| w.set_fifo_header_en(false))
        .await
        .unwrap();

    bmi.pwr_conf()
        .modify_async(|w| {
            w.set_adv_power_save(true);
            w.set_fifo_self_wake_up(false);
        })
        .await
        .unwrap();

    bmi.read_all_registers_async(|i, name, field_set_value| {
        defmt::info!("{} {} {}", i, name, field_set_value);
    })
    .await
    .unwrap();

    loop {
        let select = embassy_futures::select::select3(
            Timer::after_millis(5000),
            int1.wait_for_low(),
            int2.wait_for_low(),
        )
        .await;
        match select {
            embassy_futures::select::Either3::First(_) => defmt::warn!("Timeout"),
            embassy_futures::select::Either3::Second(_) => {}
            embassy_futures::select::Either3::Third(_) => {}
        }

        let int0 = bmi.int_status_0().read_async().await.unwrap();
        defmt::info!("{}", int0);
        let int1 = bmi.int_status_1().read_async().await.unwrap();
        defmt::info!("{}", int1);

        led.set_level((!int0.any_motion_out()).into());

        let status = bmi.status().read_async().await.unwrap();
        defmt::info!("{}", status);
        if !status.drdy_acc() {
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
