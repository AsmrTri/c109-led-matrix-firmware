// SPDX-License-Identifier: MIT
// Copyright (C) 2026 TRI ASMR
//! usb-min: C109 LED Matrix のファーム (マイコンは CH32V203RBT6)
//!
//! すること:
//! 1. USB (USBD、PA11 = D-、PA12 = D+) で PC とつながり、列挙を済ませる。設定の記述子で 500 mA を申告する
//! 2. PC が設定を選んで (SET_CONFIGURATION) 500 mA を使えるようになったら、LED マトリクスを全部点ける
//!
//! 形: メインループの無限ループで全部をポーリングする。CPU を眠らせない。割り込みは使わない。
//! - 割り込みは、main の始めに全体を止め、最後まで止めたままにする (qingke-rt は main の前に全体を許している)
//! - ch32-hal の USBD のドライバーは、割り込みのハンドラ (`InterruptHandler::on_interrupt`) で USB の状態の印
//!   (リセット・サスペンド・転送の終わり) を読んで消す。その同じ関数を、メインループから毎回呼ぶ
//! - embassy-usb の `UsbDevice::run()` の future を、実行器を使わずに、メインループから毎回 1 回ずつ poll する。
//!   起こし (waker) は何もしないもので足りる (毎回 poll するので、起こされるのを待たない)

#![no_std]
#![no_main]

use core::cell::Cell;
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Waker};

use ch32_hal as hal;
use embassy_time::{Duration, Instant};
use embassy_usb::msos::{self, windows_version};
use embassy_usb::{Builder, Handler};
use hal::gpio::{Flex, Pull, Speed};
use hal::peripherals::USBD;
use hal::usbd::{Driver, InterruptHandler};
use panic_halt as _;

// ------------------------------------------------------------------ USB の名のり (ソースの値はここだけ)

/// VID。TBD: PID を受けたら、受けた先の VID にする。いまは pid.codes の試験用の番号
const VID: u16 = 0x1209;
/// PID。TBD: 申請して受けた値にする。いまは pid.codes の試験用の番号
const PID: u16 = 0x0001;
/// 製造元の文字列。USB の文字列の記述子は UTF-16 なので、漢字もそのまま送れる
const MANUFACTURER: &str = "ASMR技研";
/// 製品の文字列。TBD
const PRODUCT: &str = "C109-LED";
/// 申告する電流 (mA)。設定の記述子の bMaxPower は、この半分 (2 mA 単位) の 250 になる
const MAX_POWER_MA: u16 = 500;
/// Windows のアプリがこの機器を探すための名前 (デバイス インターフェイスの GUID)。この機器のために作った値
const DEVICE_INTERFACE_GUIDS: &[&str] = &["{81BDC740-AF3A-4D63-9804-3FA2F2415120}"];
/// Windows が MS OS 2.0 の記述子を読むときの、ベンダー要求の番号 (bRequest)。ほかのベンダー要求は無いので、どの値でもよい
const MSOS_VENDOR_CODE: u8 = 0x01;

// ------------------------------------------------------------------ LED マトリクス

/// チャーリープレキシングの線 CP0〜CP27 の本数
const LINES: usize = 28;
/// 点ける組 (アノード, カソード) の数。28 本から違う 2 本を順に選ぶ 28 × 27 = 756 通り。
/// LED は 748 個で、残りの 8 通りには LED が無い (その間は何も光らない)
const PAIRS: usize = LINES * (LINES - 1);
/// 1 組を点ける時間。756 組で 1 回り約 9.8 ms (1 秒に約 100 回)
const SLOT: Duration = Duration::from_micros(13);

hal::bind_interrupts!(struct Irqs {
    // この割り込みは許さないので、ここのハンドラは走らない。同じ処理をメインループから呼ぶ (下の usb_irq)
    USB_LP_CAN1_RX0 => InterruptHandler<USBD>;
});

/// USBD の割り込みのハンドラと同じ仕事 (状態の印を読んで消す) を、メインループからする
fn usb_irq() {
    // SAFETY: 割り込みは全体で止めてあり、このハンドラを呼ぶのはメインループのここだけ。同時に 2 か所から走ることは無い
    unsafe {
        <InterruptHandler<USBD> as hal::interrupt::typelevel::Handler<
            hal::interrupt::typelevel::USB_LP_CAN1_RX0,
        >>::on_interrupt();
    }
}

/// PC が設定を選んだか、バスがサスペンドか、を覚える。embassy-usb が呼ぶ
struct PowerState<'a> {
    configured: &'a Cell<bool>,
    suspended: &'a Cell<bool>,
}

impl Handler for PowerState<'_> {
    /// バスのリセット: 設定は選ばれていない状態に戻る (embassy-usb はリセットで configured(false) を呼ばない)
    fn reset(&mut self) {
        self.configured.set(false);
        self.suspended.set(false);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.set(configured);
    }

    fn suspended(&mut self, suspended: bool) {
        self.suspended.set(suspended);
    }
}

/// チャーリープレキシングの線 28 本。点けるのは、いつも 1 組 (LED 1 個) だけ (この基板の設計の前提)
struct Matrix<'d> {
    pins: [Flex<'d>; LINES],
    lit: Option<(usize, usize)>,
}

impl<'d> Matrix<'d> {
    /// 全部の線を Hi-Z (入力、プルなし) にして作る
    fn new(mut pins: [Flex<'d>; LINES]) -> Self {
        for pin in pins.iter_mut() {
            pin.set_as_input(Pull::None);
        }
        Self { pins, lit: None }
    }

    /// アノード a・カソード k の組を点ける。前の組は先に Hi-Z に戻す
    fn on(&mut self, a: usize, k: usize) {
        self.off();
        // レベルを決めてから出力にする
        self.pins[k].set_low();
        self.pins[k].set_as_output(Speed::Medium);
        self.pins[a].set_high();
        self.pins[a].set_as_output(Speed::Medium);
        self.lit = Some((a, k));
    }

    /// 点いている組を Hi-Z に戻す
    fn off(&mut self) {
        if let Some((a, k)) = self.lit.take() {
            self.pins[a].set_as_input(Pull::None);
            self.pins[k].set_as_input(Pull::None);
        }
    }
}

/// i 番目 (0〜755) の組。アノードは i / 27 番の線、カソードは残りの 27 本の中の i % 27 番目 (アノードの線を飛ばす)
fn pair(i: usize) -> (usize, usize) {
    let a = i / (LINES - 1);
    let j = i % (LINES - 1);
    (a, if j < a { j } else { j + 1 })
}

#[qingke_rt::entry]
fn main() -> ! {
    // 割り込みを全体で止める。最後まで許さない
    qingke::register::gintenr::set_disable();

    // 144 MHz (HSI × 18)。USB の 48 MHz は PLL の出力の 3 分周
    let p = hal::init(hal::Config {
        rcc: hal::rcc::Config::SYSCLK_FREQ_144MHZ_HSI,
        ..Default::default()
    });

    // 線 CP0〜CP27 を、この順に並べる。全部を点けるだけなので順は効かないが、追いやすいように CP の順にした。
    // 出どころ: 基板の回路図 (公開していない) の、CP0〜CP27 の線と MCU (LQFP64) のピンのつながり。
    // PB2 (BOOT1)、ユーザー LED の PA10・PA15、PB6・PB7 は触らない
    let mut matrix = Matrix::new([
        Flex::new(p.PC0),  // CP0  (ピン 8)
        Flex::new(p.PC2),  // CP1  (ピン 10)
        Flex::new(p.PC1),  // CP2  (ピン 9)
        Flex::new(p.PA0),  // CP3  (ピン 14)
        Flex::new(p.PC3),  // CP4  (ピン 11)
        Flex::new(p.PA3),  // CP5  (ピン 17)
        Flex::new(p.PA1),  // CP6  (ピン 15)
        Flex::new(p.PA5),  // CP7  (ピン 21)
        Flex::new(p.PA4),  // CP8  (ピン 20)
        Flex::new(p.PA6),  // CP9  (ピン 22)
        Flex::new(p.PA7),  // CP10 (ピン 23)
        Flex::new(p.PB0),  // CP11 (ピン 26)
        Flex::new(p.PC5),  // CP12 (ピン 25)
        Flex::new(p.PB11), // CP13 (ピン 30)
        Flex::new(p.PB10), // CP14 (ピン 29)
        Flex::new(p.PB13), // CP15 (ピン 34)
        Flex::new(p.PB12), // CP16 (ピン 33)
        Flex::new(p.PC6),  // CP17 (ピン 37)
        Flex::new(p.PB15), // CP18 (ピン 36)
        Flex::new(p.PC8),  // CP19 (ピン 39)
        Flex::new(p.PC7),  // CP20 (ピン 38)
        Flex::new(p.PA9),  // CP21 (ピン 42)
        Flex::new(p.PA8),  // CP22 (ピン 41)
        Flex::new(p.PA2),  // CP23 (ピン 16)
        Flex::new(p.PC4),  // CP24 (ピン 24)
        Flex::new(p.PB1),  // CP25 (ピン 27)
        Flex::new(p.PB14), // CP26 (ピン 35)
        Flex::new(p.PC9),  // CP27 (ピン 40)
    ]);

    // ---- USB の機器 ----
    // D+ の内蔵のプルアップは Driver::new が入れる (ここで PC から見えるようになる)
    let driver = Driver::new(p.USBD, Irqs, p.PA12, p.PA11);

    let mut config = embassy_usb::Config::new(VID, PID);
    config.manufacturer = Some(MANUFACTURER);
    config.product = Some(PRODUCT);
    config.max_power = MAX_POWER_MA; // バスから電源をとる (self_powered は false のまま) ので bmAttributes は 0x80
    config.max_packet_size_0 = 64;
    // 機器の種類はインターフェースで決める (0x00)。機能は 1 つだけなので、IAD は付けない
    config.device_class = 0x00;
    config.device_sub_class = 0x00;
    config.device_protocol = 0x00;
    config.composite_with_iads = false;

    let mut config_descriptor = [0u8; 32];
    let mut bos_descriptor = [0u8; 64];
    let mut msos_descriptor = [0u8; 256];
    let mut control_buf = [0u8; 64];

    let configured = Cell::new(false);
    let suspended = Cell::new(false);
    let mut power = PowerState {
        configured: &configured,
        suspended: &suspended,
    };

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );
    builder.handler(&mut power);

    // Windows が、ドライバーを入れなくても WinUSB を当てるための印 (MS OS 2.0 の記述子)。
    // Windows では、当たったドライバーが設定を選ぶ。WinUSB が当たると設定が選ばれ、500 mA を使えるようになる
    builder.msos_descriptor(windows_version::WIN8_1, MSOS_VENDOR_CODE);
    builder.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    builder.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        msos::PropertyData::RegMultiSz(DEVICE_INTERFACE_GUIDS),
    ));

    // ベンダー固有のインターフェース 1 つ。エンドポイント 0 のほかは持たない (データの通信をしない)
    {
        let mut function = builder.function(0xFF, 0x00, 0x00);
        let mut interface = function.interface();
        let _alt = interface.alt_setting(0xFF, 0x00, 0x00, None);
    }

    let mut usb = builder.build();
    let mut usb_run = pin!(usb.run());
    let mut cx = Context::from_waker(Waker::noop());

    let mut next = 0usize;
    let mut deadline = Instant::now();

    loop {
        // USB: 割り込みのハンドラと同じ仕事をしてから、future を 1 回だけ進める
        usb_irq();
        let _ = usb_run.as_mut().poll(&mut cx);

        // LED: PC が設定を選んでいて、サスペンドでない間 (500 mA を使える間) だけ点ける
        if configured.get() && !suspended.get() {
            let now = Instant::now();
            if matrix.lit.is_none() || now >= deadline {
                let (a, k) = pair(next);
                matrix.on(a, k);
                next = (next + 1) % PAIRS;
                deadline = now + SLOT;
            }
        } else {
            matrix.off();
        }
    }
}
