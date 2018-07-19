all: build_tx flash_tx

clean:
	rm -r target

tx:
	cargo build --release --bin controller
	arm-none-eabi-objcopy -SO binary target/thumbv7m-none-eabi/release/controller target/thumbv7m-none-eabi/release/controller.bin
	st-flash --reset write target/thumbv7m-none-eabi/release/controller.bin 0x08000000

rx:
	cargo build --release --bin receiver
	arm-none-eabi-objcopy -SO binary target/thumbv7m-none-eabi/release/receiver target/thumbv7m-none-eabi/release/receiver.bin
	st-flash --reset write target/thumbv7m-none-eabi/release/receiver.bin 0x08000000

debug_tx:
	cargo build --release --bin controller
	arm-none-eabi-gdb target/thumbv7m-none-eabi/release/controller

debug_rx:
	cargo build --release --bin receiver
	arm-none-eabi-gdb target/thumbv7m-none-eabi/release/receiver

openocd:
	openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg
