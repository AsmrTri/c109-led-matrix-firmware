# vendor

ほかの人が作ったソースの写し。ここは書き換えない (直すときは、下に直した所を書く)。

使う所: このリポジトリのファーム (Cargo.toml の path で、この写しを指す)。

## ch32-hal

- 出どころ: https://github.com/ch32-rs/ch32-hal の 71a2985804962e248efc09e3bb4f21d4459b50e8 (2026-06-09)。cargo の git の写しから
  取り出した (examples、ci、.github、.gitignore、rustfmt.toml は除いた)
- ライセンス: MIT か Apache-2.0 (LICENSE-MIT、LICENSE-APACHE)
- 写した理由: この版をそのまま使うと、CH32V203RBT6 でビルドが通らない。ch32-metapac は RBT6 の TIM5 を 32 ビットのタイマー
  (`timer::Gptm32`) としているが、ch32-hal は 32 ビットのタイマーの扱い (`TimerBits::Bits32` と、それを使う所) を CH32L1 と
  CH32V208 のときだけ作る。上流の新しい版 (0766331、ae4eb1e) も同じだった
- 直した所 1 (ビルドを通すため): `src/timer/mod.rs` と `src/timer/low_level.rs` の `#[cfg(any(ch32l1, ch32v208))]` 7 か所を
  `#[cfg(any(ch32l1, ch32v208, peri_tim5))]` にした (TIM5 がある石でも 32 ビットのタイマーの扱いを作る)
- 直した所 2 (USB の 1 回の poll を短くするため、2026-11-13): `src/usbd.rs` の `EndpointBuffer::read` と `write`
  (USB の受け渡しの置き場 USBRAM と、受け取った・送るバイトの間の写し)
  - 理由: 元は 2 バイトごとに `copy_from_slice` を呼び、その先で `memcpy` が 2 バイトを写していた。直す前の ELF では、
    64 バイトの写しに約 1,600〜1,900 クロック (144 MHz で約 11〜13 µs) かかった
  - 直し方: `chunks_exact` で 2 バイトずつ取り、u16 を直接組み立てて (下位のバイトが先) USBRAM に書く。読むときは u16 を読んで
    2 バイトに分ける。奇数の長さの最後の 1 バイトは、書くときは上位を 0 にした u16、読むときは下位のバイトだけ。
    読み書きする番地 (`USBRAM.mem(addr / 2 + i)`) と値は元と同じ (直す前と後のソースを読み合わせて確かめた)
  - 命令の数 (release、opt-level z、LTO の ELF を llvm-objdump で逆アセンブルして数えた):
    - 関数の中の命令: `write_data` 63 → 55、`read_data` 78 → 60。直した後は `copy_from_slice` も `memcpy` も呼ばない
    - 2 バイトごとの繰り返し: 書く 11 命令、読む 10 命令。直す前は、繰り返しの 22 命令に、範囲の確かめ (4)、`copy_from_slice` (6)、
      `memcpy` の短い道 (約 20) を足して約 52 命令
    - 64 バイトでは、書く約 380 命令、読む約 360 命令 (直す前は約 1,700)。USBRAM の読み書きの待ちは計っていないので、
      時間は 144 MHz で約 3 µs の見込み
- ほかは変えていない
