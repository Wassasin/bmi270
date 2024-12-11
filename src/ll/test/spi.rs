use embedded_hal_mock::eh1::spi::{Mock, Transaction};

use crate::ll;

fn regw(register: u8, values: &[u8]) -> Vec<Transaction<u8>> {
    vec![
        Transaction::transaction_start(),
        Transaction::write(register),
        Transaction::write_vec(values.to_vec()),
        Transaction::transaction_end(),
    ]
}

fn regr(register: u8, values: &[u8]) -> Vec<Transaction<u8>> {
    vec![
        Transaction::transaction_start(),
        Transaction::write_vec(vec![register | 0b1000_0000, 0x00]),
        Transaction::read_vec(values.to_vec()),
        Transaction::transaction_end(),
    ]
}

#[async_std::test]
async fn example() {
    let expectations = [
        regr(0x00, &[0x24]),
        regw(0x7E, &[0xB6]),
        regr(0x7C, &[0x03]),
        regw(0x7C, &[0x02]),
    ];

    let mut spi = Mock::new(expectations.iter().flatten());

    let mut ll = ll::Device::new(ll::spi::DeviceInterface::new(&mut spi));
    assert_eq!(ll.chip_id().read_async().await.unwrap().chip_id(), 0x24);

    ll.cmd()
        .write_async(|w| w.set_cmd(ll::Command::Softreset))
        .await
        .unwrap();

    ll.pwr_conf()
        .modify_async(|w| w.set_adv_power_save(false))
        .await
        .unwrap();

    spi.done();
}

#[async_std::test]
async fn init() {
    let expectations = [regw(0x5B, &[0x0A, 0xC8]), regw(0x5E, &[0x01, 0x02, 0x03])];

    let mut spi = Mock::new(expectations.iter().flatten());

    let mut ll = ll::Device::new(ll::spi::DeviceInterface::new(&mut spi));

    let addr = 6420;
    let haddr = (addr / 2) as u16;
    ll.init_addr()
        .write_async(|w| {
            w.set_init_addr_0((haddr & 0x0F) as u8);
            w.set_init_addr_1((haddr >> 4) as u8);
        })
        .await
        .unwrap();

    ll.init_data()
        .write_async(&[0x01, 0x02, 0x03])
        .await
        .unwrap();

    spi.done();
}
