all: build flash

clean:
	rm -r target

build:
	cargo build --release
	arm-none-eabi-objcopy -SO binary target/thumbv7m-none-eabi/release/mremote target/thumbv7m-none-eabi/release/mremote.bin

flash:
	st-flash --reset write target/thumbv7m-none-eabi/release/mremote.bin 0x08000000

debug:
	cargo build --release
	arm-none-eabi-gdb target/thumbv7m-none-eabi/release/mremote

openocd:
	openocd -f interface/stlink-v2.cfg -f target/stm32f1x.cfg
