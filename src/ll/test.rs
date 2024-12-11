use embedded_hal_mock::eh1::i2c::{Mock, Transaction};

use crate::ll;

fn regw(register: u8, values: &[u8]) -> Transaction {
    let mut expected = vec![register];
    expected.extend_from_slice(values);
    Transaction::write(ll::i2c::Address::Default as u8, expected)
}

fn regr(register: u8, values: &[u8]) -> Transaction {
    Transaction::write_read(
        ll::i2c::Address::Default as u8,
        vec![register],
        Vec::from(values),
    )
}

#[async_std::test]
async fn example() {
    let expectations = [
        regr(0x00, &[0x24]),
        regw(0x7E, &[0xB6]),
        regr(0x7C, &[0x03]),
        regw(0x7C, &[0x02]),
    ];

    let mut i2c = Mock::new(&expectations);

    let mut ll = ll::Device::new(ll::i2c::DeviceInterface::new(
        &mut i2c,
        ll::i2c::Address::Default,
    ));
    assert_eq!(ll.chip_id().read_async().await.unwrap().chip_id(), 0x24);

    ll.cmd()
        .write_async(|w| w.set_cmd(ll::Command::Softreset))
        .await
        .unwrap();

    ll.pwr_conf()
        .modify_async(|w| w.set_adv_power_save(false))
        .await
        .unwrap();

    i2c.done();
}

#[async_std::test]
async fn init() {
    let expectations = [regw(0x5B, &[0x0A, 0xC8]), regw(0x5E, &[0x01, 0x02, 0x03])];

    let mut i2c = Mock::new(&expectations);

    let mut ll = ll::Device::new(ll::i2c::DeviceInterface::new(
        &mut i2c,
        ll::i2c::Address::Default,
    ));

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

    i2c.done();
}
