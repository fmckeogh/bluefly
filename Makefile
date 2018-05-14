all: build flash

clean:
	rm -r target

build:
	cargo build --release
	arm-none-eabi-objcopy -SO binary target/thumbv7m-none-eabi/release/mremote target/thumbv7m-none-eabi/release/mremote.bin
	b2sum -l 128 target/thumbv7m-none-eabi/release/mremote
	b2sum -l 128 target/thumbv7m-none-eabi/release/mremote.bin

flash:
	st-flash --reset write target/thumbv7m-none-eabi/release/mremote.bin 0x08000000
