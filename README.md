# usb-min: firmware for the C109 LED Matrix

English first. 日本語は後半にあります。

## What this is

usb-min is firmware for the C109 LED Matrix, a USB-powered display gadget with a 34 x 22 matrix of green LEDs
(748 LEDs). The MCU is a WCH CH32V203RBT6 (RISC-V). The firmware is written in Rust. This repository contains the
firmware; the hardware design of the board is not published.

What it does:

- On USB it enumerates as a bus-powered device and asks for 500 mA in its configuration descriptor
- It presents one vendor-specific interface, with no endpoints besides endpoint 0, and MS OS 2.0 descriptors (the
  compatible ID WINUSB and a DeviceInterfaceGUID), so that Windows binds WinUSB to it without a driver package
- When the PC has selected the configuration (SET_CONFIGURATION), so that the gadget may draw 500 mA, it lights all
  748 LEDs: one LED at a time, in turn, about 100 times a second. While the bus is suspended, the LEDs are off

## Licences

Copyright (C) 2026 TRI ASMR

| Path | What it is | Licence | Full text |
|---|---|---|---|
| `usb-min/` | this firmware | MIT | `LICENSE` |
| `vendor/ch32-hal/` | a copy of ch32-hal with small changes | MIT OR Apache-2.0 (the same as the original; the changes are under the same terms) | `vendor/ch32-hal/LICENSE-MIT`, `vendor/ch32-hal/LICENSE-APACHE` |

Every Rust source file under `usb-min/` (`src/main.rs`) starts with an `SPDX-License-Identifier` line and a
copyright line.

`vendor/ch32-hal/` was copied from https://github.com/ch32-rs/ch32-hal at commit
71a2985804962e248efc09e3bb4f21d4459b50e8 (2026-06-09). Changed files: `src/timer/mod.rs` and `src/timer/low_level.rs`
(the 32-bit timer support is also built for chips with TIM5, so that it builds for the CH32V203RBT6) and `src/usbd.rs`
(`EndpointBuffer::read` and `write` copy 16-bit words directly). The details are in `vendor/README.md` (in Japanese).

### Third-party crates built into the firmware

They are not in this repository; cargo downloads them. The list is from `usb-min/Cargo.lock`
(`cargo tree -e normal,no-proc-macro --target riscv32imc-unknown-none-elf`): 54 crates besides ch32-hal and usb-min
itself.

- MIT OR Apache-2.0 (48): bit_field 0.10.3, cfg-if 1.0.4, ch32-metapac 0.1.0 (from git, rev 23d66721),
  critical-section 1.2.0, embassy-executor-timer-queue 0.1.0, embassy-futures 0.1.2, embassy-hal-internal 0.3.0,
  embassy-net-driver 0.2.0, embassy-net-driver-channel 0.3.2 and 0.4.0, embassy-sync 0.7.2 and 0.8.0,
  embassy-time 0.5.1, embassy-time-driver 0.2.2, embassy-time-queue-utils 0.3.2, embassy-usb 0.5.1,
  embassy-usb-driver 0.2.0, embedded-can 0.4.1, embedded-hal 0.2.7 and 1.0.0, embedded-hal-async 1.0.0,
  embedded-hal-nb 1.0.0, embedded-io 0.6.1 and 0.7.1, embedded-io-async 0.6.1 and 0.7.0, embedded-storage 0.3.1,
  futures 0.3.32, futures-channel 0.3.32, futures-core 0.3.32, futures-io 0.3.32, futures-sink 0.3.32,
  futures-task 0.3.32, futures-util 0.3.32, hash32 0.3.1, heapless 0.8.0 and 0.9.3, nb 0.1.3 and 1.1.0,
  num-traits 0.2.19, panic-halt 1.0.0, pin-project-lite 0.2.17, qingke 0.7.0, qingke-rt 0.7.0, rand_core 0.6.4,
  sdio-host 0.5.0, stable_deref_trait 1.2.1, vcell 0.1.3
- ISC (3): riscv 0.12.1, riscv 0.15.0, riscv-pac 0.2.0
- MIT (1): void 1.0.2
- Unlicense OR MIT (1): byteorder 1.5.0
- MPL-2.0 or later (1): bitmaps 3.2.1 (only the OTG and USBHS drivers of ch32-hal use it; usb-min does not use those
  drivers)

The parts of the Rust standard library in the firmware (core, compiler_builtins) are under MIT OR Apache-2.0 (the
parts of compiler_builtins that come from LLVM are under Apache-2.0 WITH LLVM-exception).

Not built into the firmware:

- Used only while building (procedural macros, build scripts and what they use, from the same `Cargo.lock`;
  `cargo tree -e normal,build` lists them besides the crates above): autocfg 1.5.1, document-features 0.2.12,
  futures-macro 0.3.32, litrs 1.0.0, paste 1.0.15, proc-macro-error 1.0.4, proc-macro-error-attr 1.0.4,
  proc-macro2 1.0.106, qingke-rt-macros 0.7.0, quote 1.0.46, riscv-macros 0.1.0 and 0.3.0, syn 1.0.109 and 2.0.119,
  version_check 0.9.5 (MIT OR Apache-2.0), unicode-ident 1.0.24 ((MIT OR Apache-2.0) AND Unicode-3.0)

## Hardware

The design of the board (schematic and layout) is not published. This repository contains the firmware.
The pin assignment (USB on PA11 and PA12, the 28 LED lines CP0 to CP27), the USB vendor and product IDs and the
strings are in `usb-min/src/main.rs`.

## Build (Windows, cmd)

You need Rust (rustup, stable; checked with rustc 1.96.1) and the target of the MCU:

```cmd
rustup target add riscv32imc-unknown-none-elf
```

Download the crates (`cargo fetch`, needed only the first time) and build. Replace `C:\work\c109-led` with the folder
of your copy:

```cmd
cd /d C:\work\c109-led\usb-min
cargo fetch
cargo build --release
copy /y target\riscv32imc-unknown-none-elf\release\usb-min usb-min.elf
```

cargo makes the ELF file `usb-min\target\riscv32imc-unknown-none-elf\release\usb-min` (without an extension). The
last line copies it to `usb-min\usb-min.elf`, the file to write to the gadget.

## Install the firmware on the gadget

The firmware is written over USB with the bootloader built into the CH32V203 (USB ISP) and wchisp
(https://github.com/ch32-rs/wchisp). The board has no SWD connector.

1. Build the firmware (above). `usb-min\usb-min.elf` is made
2. Remove the back cover and connect the USB-C port of the MCU board to the PC
3. Hold down BOOT0 (SW4), press and release reset (SW5), then release BOOT0. The MCU starts its built-in bootloader
   (the LED matrix stays dark)
4. Write the firmware (erase, write, verify and reset):

   ```cmd
   wchisp flash C:\work\c109-led\usb-min\usb-min.elf
   ```

5. The gadget resets and runs the new firmware. When the PC has selected the configuration, all the LEDs light
   (dimly, because only one LED is lit at a time)

If it does not work, start again from step 3.

## Notes

- The comments in the source are in Japanese
- Some comments refer to the board design, which is not published (for example, the schematic that the pin numbers come
  from)

---

# usb-min: C109 LED Matrix のファーム

## これは何か

usb-min は、C109 LED Matrix (横 34 × 縦 22 の緑の LED 748 個を並べた、USB給電のLED表示デバイス) のファーム。マイコンは
WCH の CH32V203RBT6 (RISC-V)。Rust で書いた。このリポジトリにはファームを置く。基板のハードの設計は公開していない。

すること:

- USB で、バスから電源をとる機器として列挙を済ませ、設定の記述子で 500 mA を申告する
- ベンダー固有のインターフェースを 1 つ出す (エンドポイントは 0 番だけ)。MS OS 2.0 の記述子 (互換 ID の WINUSB と、
  DeviceInterfaceGUID) を付けて、ドライバーを入れなくても Windows が WinUSB を当てるようにする
- PC が設定を選んで (SET_CONFIGURATION)、500 mA を使えるようになったら、LED 748 個を全部点ける (1 個ずつ順に、1 秒に約 100 回り)。
  バスがサスペンドの間は消す

## ライセンス

Copyright (C) 2026 TRI ASMR (ASMR技研)

| 場所 | 中身 | ライセンス | 全文 |
|---|---|---|---|
| `usb-min/` | このファーム | MIT | `LICENSE` |
| `vendor/ch32-hal/` | ch32-hal の写し (少し直した) | MIT か Apache-2.0 (元と同じ。直した所も同じ条件) | `vendor/ch32-hal/LICENSE-MIT`、`vendor/ch32-hal/LICENSE-APACHE` |

`usb-min/` の下の Rust のソース (`src/main.rs`) は、どれも頭に `SPDX-License-Identifier` の行と、著作権の行がある。

`vendor/ch32-hal/` は、https://github.com/ch32-rs/ch32-hal の 71a2985804962e248efc09e3bb4f21d4459b50e8 (2026-06-09) の写し。
直したファイル: `src/timer/mod.rs` と `src/timer/low_level.rs` (32 ビットのタイマーの扱いを、TIM5 がある石でも作る。
CH32V203RBT6 でビルドを通すため) と、`src/usbd.rs` (`EndpointBuffer::read` と `write` で、16 ビットずつ直接写す)。
詳しくは `vendor/README.md`。

### ファームに入る他人の crate

このリポジトリには入っていない (cargo が取ってくる)。`usb-min/Cargo.lock` から
`cargo tree -e normal,no-proc-macro --target riscv32imc-unknown-none-elf` で数えた。ch32-hal と usb-min 自身のほかに 54 件。

- MIT か Apache-2.0 (48 件): 上の英語の一覧と同じ
- ISC (3 件): riscv 0.12.1、riscv 0.15.0、riscv-pac 0.2.0
- MIT (1 件): void 1.0.2
- Unlicense か MIT (1 件): byteorder 1.5.0
- MPL-2.0 以降 (1 件): bitmaps 3.2.1 (ch32-hal の OTG と USBHS のドライバーだけが使う。usb-min はそのドライバーを使わない)

ファームに入る Rust の標準の部品 (core、compiler_builtins) は MIT か Apache-2.0 (compiler_builtins の LLVM から来た部分は
LLVM の例外つきの Apache-2.0)。

ファームに入らない物:

- ビルドの間だけ使う物 (proc-macro と、build.rs (ビルドのときに回るスクリプト) と、それらが使う物。同じ `Cargo.lock`。
  `cargo tree -e normal,build` で、上の一覧のほかに出る): autocfg 1.5.1、document-features 0.2.12、futures-macro 0.3.32、
  litrs 1.0.0、paste 1.0.15、proc-macro-error 1.0.4、proc-macro-error-attr 1.0.4、proc-macro2 1.0.106、qingke-rt-macros 0.7.0、
  quote 1.0.46、riscv-macros 0.1.0 と 0.3.0、syn 1.0.109 と 2.0.119、version_check 0.9.5 (MIT か Apache-2.0)、
  unicode-ident 1.0.24 ((MIT か Apache-2.0) と Unicode-3.0)

## 基板

基板の設計 (回路図と配線) は公開していない。このリポジトリにはファームを置く。ピンの割り当て (USB の PA11・PA12、28 本の LED の線
CP0〜CP27) と、USB の VID・PID と文字列は `usb-min/src/main.rs` にある。

## ビルドする (Windows、cmd)

Rust (rustup、stable。rustc 1.96.1 で確かめた) と、マイコンの target が要る:

```cmd
rustup target add riscv32imc-unknown-none-elf
```

crate を取ってきて (`cargo fetch`。初めのときだけ要る)、ビルドする。`C:\work\c109-led` は、このリポジトリを置いたフォルダに替える:

```cmd
cd /d C:\work\c109-led\usb-min
cargo fetch
cargo build --release
copy /y target\riscv32imc-unknown-none-elf\release\usb-min usb-min.elf
```

cargo は ELF のファイル `usb-min\target\riscv32imc-unknown-none-elf\release\usb-min` (拡張子なし) を作る。最後の行で、それを
`usb-min\usb-min.elf` に写す。機器に書き込むのは、このファイル。

## 機器に書き込む

CH32V203 の中の書き込みの仕組み (USB ISP) と wchisp (https://github.com/ch32-rs/wchisp) で、USB から書き込む。SWD の端子は無い。

1. ファームをビルドする (上)。`usb-min\usb-min.elf` ができる
2. 裏ぶたを外し、マイコン基板の USB-C を PC に挿す
3. BOOT0 (SW4) を押したまま、リセット (SW5) を押して離し、それから BOOT0 を離す (チップの中の書き込みの仕組みに入る。
   この間、LED は消えている)
4. 書き込む (消去、書き込み、確かめ、リセットまで自動):

   ```cmd
   wchisp flash C:\work\c109-led\usb-min\usb-min.elf
   ```

5. 終わるとリセットされ、新しいファームが走る。PC が設定を選ぶと、LED が全部点く (1 個ずつ点けるので暗い)

うまくいかないときは、3 からやり直す。

## ほか

- ソースのコメントは日本語
- コメントには、公開していない基板の設計を指す所がある (ピン番号の出どころの回路図など)
